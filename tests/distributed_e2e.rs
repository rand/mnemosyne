use mnemosyne_core::orchestration::messages::{ExecutorMessage, AgentMessage};
use mnemosyne_core::orchestration::network::{NetworkLayer, MessageRouter};
use mnemosyne_core::orchestration::network::router::LocalAgent;
use mnemosyne_core::orchestration::state::{WorkItem, Phase};
use mnemosyne_core::launcher::agents::AgentRole;
use ractor::{Actor, ActorRef, ActorProcessingErr};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::mpsc;
use std::time::Duration;

struct MockExecutorActor {
    tx: mpsc::Sender<WorkItem>,
}

#[async_trait]
impl Actor for MockExecutorActor {
    type Msg = ExecutorMessage;
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ExecutorMessage::ExecuteWork(item) => {
                 let _ = self.tx.send(item).await;
            }
            _ => {}
        }
        Ok(())
    }
}

async fn spawn_test_node() -> anyhow::Result<(Arc<NetworkLayer>, Arc<MessageRouter>)> {
    let layer = Arc::new(NetworkLayer::new().await?);
    layer.start().await?;
    let router = layer.router();
    Ok((layer, router))
}

#[tokio::test]
#[ignore] // Flaky in restricted test environments due to P2P connection setup
async fn test_distributed_execution_flow() -> anyhow::Result<()> {
    // Force binding to localhost for test
    std::env::set_var("MNEMOSYNE_TEST_BIND_ADDR", "127.0.0.1:0");

    // 1. Spawn Node A (Orchestrator) and Node B (Executor)
    let (node_a, router_a) = spawn_test_node().await?;
    let (node_b, router_b) = spawn_test_node().await?;
    
    // 2. Setup Node B with a Mock Executor
    let (tx, mut rx) = mpsc::channel(1);
    let mock_executor = MockExecutorActor { tx };
    let (executor_ref, _) = Actor::spawn(None, mock_executor, ()).await.unwrap();
    
    // Register local executor on Node B
    router_b.register_local(AgentRole::Executor, LocalAgent::Executor(executor_ref)).await;
    
    // 3. Connect Node A to Node B
    // Get Node B's invite ticket
    let ticket = node_b.create_invite().await?;
    
    // Node A joins Node B
    node_a.join_peer(&ticket).await?;
    println!("Node A joined Node B with ticket: {}", ticket);
    
    // Wait a bit for connection to be established
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 4. Register Node B as remote executor on Node A
    let node_b_id = node_b.node_id().await.unwrap();
    router_a.register_remote(AgentRole::Executor, node_b_id).await;
    
    // 5. Send work from Node A
    let work_item = WorkItem::new(
        "Test distributed work".to_string(),
        AgentRole::Executor,
        Phase::PlanToArtifacts,
        1
    );
    let work_item_id = work_item.id.clone();
    let message = AgentMessage::Executor(Box::new(ExecutorMessage::ExecuteWork(work_item)));
    
    // Route message with retry
    let mut attempts = 0;
    loop {
        match router_a.route(AgentRole::Executor, message.clone()).await {
            Ok(_) => {
                println!("Message routed successfully");
                break;
            }
            Err(e) => {
                attempts += 1;
                if attempts > 5 {
                    return Err(anyhow::anyhow!("Failed to route message after 5 attempts: {}", e));
                }
                println!("Route failed (attempt {}): {}. Retrying...", attempts, e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    // 6. Verify Node B received the work
    let received = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
    
    assert!(received.is_ok(), "Timed out waiting for message");
    let received_item = received.unwrap().unwrap();
    assert_eq!(received_item.id, work_item_id);
    
    // Cleanup
    node_a.stop().await?;
    node_b.stop().await?;
    
    Ok(())
}
