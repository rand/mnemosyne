//! Type conversions between Protobuf and internal types

use crate::rpc::generated;
use crate::types::{
    LinkType as InternalLinkType, MemoryNote as InternalMemoryNote,
    MemoryType as InternalMemoryType, Namespace as InternalNamespace,
};
use tonic::Status;

/// Convert Protobuf Namespace to internal Namespace
pub fn namespace_from_proto(ns: generated::Namespace) -> Result<InternalNamespace, Status> {
    match ns.namespace {
        Some(generated::namespace::Namespace::Global(_)) => Ok(InternalNamespace::Global),
        Some(generated::namespace::Namespace::Project(p)) => {
            Ok(InternalNamespace::Project { name: p.name })
        }
        Some(generated::namespace::Namespace::Session(s)) => Ok(InternalNamespace::Session {
            project: s.project,
            session_id: s.session_id,
        }),
        None => Err(Status::invalid_argument("Namespace is required")),
    }
}

/// Convert internal Namespace to Protobuf Namespace
pub fn namespace_to_proto(ns: InternalNamespace) -> generated::Namespace {
    let namespace = match ns {
        InternalNamespace::Global => Some(generated::namespace::Namespace::Global(
            generated::GlobalNamespace {},
        )),
        InternalNamespace::Project { name } => Some(generated::namespace::Namespace::Project(
            generated::ProjectNamespace { name },
        )),
        InternalNamespace::Session {
            project,
            session_id,
        } => Some(generated::namespace::Namespace::Session(
            generated::SessionNamespace {
                project,
                session_id,
            },
        )),
    };
    generated::Namespace { namespace }
}

/// Convert Protobuf MemoryType to internal MemoryType
pub fn memory_type_from_proto(mt: i32) -> InternalMemoryType {
    match generated::MemoryType::try_from(mt) {
        Ok(generated::MemoryType::ArchitectureDecision) => InternalMemoryType::ArchitectureDecision,
        Ok(generated::MemoryType::CodePattern) => InternalMemoryType::CodePattern,
        Ok(generated::MemoryType::BugFix) => InternalMemoryType::BugFix,
        Ok(generated::MemoryType::Configuration) => InternalMemoryType::Configuration,
        Ok(generated::MemoryType::Constraint) => InternalMemoryType::Constraint,
        Ok(generated::MemoryType::Entity) => InternalMemoryType::Entity,
        Ok(generated::MemoryType::Insight) => InternalMemoryType::Insight,
        Ok(generated::MemoryType::Reference) => InternalMemoryType::Reference,
        Ok(generated::MemoryType::Preference) => InternalMemoryType::Preference,
        Ok(generated::MemoryType::Task) => InternalMemoryType::Task,
        Ok(generated::MemoryType::AgentEvent) => InternalMemoryType::AgentEvent,
        Ok(generated::MemoryType::Constitution) => InternalMemoryType::Constitution,
        Ok(generated::MemoryType::FeatureSpec) => InternalMemoryType::FeatureSpec,
        Ok(generated::MemoryType::ImplementationPlan) => InternalMemoryType::ImplementationPlan,
        Ok(generated::MemoryType::TaskBreakdown) => InternalMemoryType::TaskBreakdown,
        Ok(generated::MemoryType::QualityChecklist) => InternalMemoryType::QualityChecklist,
        Ok(generated::MemoryType::Clarification) => InternalMemoryType::Clarification,
        _ => InternalMemoryType::Insight, // Default fallback
    }
}

/// Convert internal MemoryType to Protobuf MemoryType
pub fn memory_type_to_proto(mt: InternalMemoryType) -> i32 {
    let proto_type = match mt {
        InternalMemoryType::ArchitectureDecision => generated::MemoryType::ArchitectureDecision,
        InternalMemoryType::CodePattern => generated::MemoryType::CodePattern,
        InternalMemoryType::BugFix => generated::MemoryType::BugFix,
        InternalMemoryType::Configuration => generated::MemoryType::Configuration,
        InternalMemoryType::Constraint => generated::MemoryType::Constraint,
        InternalMemoryType::Entity => generated::MemoryType::Entity,
        InternalMemoryType::Insight => generated::MemoryType::Insight,
        InternalMemoryType::Reference => generated::MemoryType::Reference,
        InternalMemoryType::Preference => generated::MemoryType::Preference,
        InternalMemoryType::Task => generated::MemoryType::Task,
        InternalMemoryType::AgentEvent => generated::MemoryType::AgentEvent,
        InternalMemoryType::Constitution => generated::MemoryType::Constitution,
        InternalMemoryType::FeatureSpec => generated::MemoryType::FeatureSpec,
        InternalMemoryType::ImplementationPlan => generated::MemoryType::ImplementationPlan,
        InternalMemoryType::TaskBreakdown => generated::MemoryType::TaskBreakdown,
        InternalMemoryType::QualityChecklist => generated::MemoryType::QualityChecklist,
        InternalMemoryType::Clarification => generated::MemoryType::Clarification,
    };
    proto_type as i32
}

/// Convert Protobuf LinkType to internal LinkType
pub fn link_type_from_proto(lt: i32) -> InternalLinkType {
    match generated::LinkType::try_from(lt) {
        Ok(generated::LinkType::Extends) => InternalLinkType::Extends,
        Ok(generated::LinkType::BuildsUpon) => InternalLinkType::BuildsUpon,
        Ok(generated::LinkType::Contradicts) => InternalLinkType::Contradicts,
        Ok(generated::LinkType::Implements) => InternalLinkType::Implements,
        Ok(generated::LinkType::References) => InternalLinkType::References,
        Ok(generated::LinkType::ReferencedBy) => InternalLinkType::ReferencedBy,
        Ok(generated::LinkType::Clarifies) => InternalLinkType::Clarifies,
        Ok(generated::LinkType::Supersedes) => InternalLinkType::Supersedes,
        _ => InternalLinkType::References, // Default fallback
    }
}

/// Convert internal LinkType to Protobuf LinkType
pub fn link_type_to_proto(lt: InternalLinkType) -> i32 {
    let proto_type = match lt {
        InternalLinkType::Extends => generated::LinkType::Extends,
        InternalLinkType::BuildsUpon => generated::LinkType::BuildsUpon,
        InternalLinkType::Contradicts => generated::LinkType::Contradicts,
        InternalLinkType::Implements => generated::LinkType::Implements,
        InternalLinkType::References => generated::LinkType::References,
        InternalLinkType::ReferencedBy => generated::LinkType::ReferencedBy,
        InternalLinkType::Clarifies => generated::LinkType::Clarifies,
        InternalLinkType::Supersedes => generated::LinkType::Supersedes,
    };
    proto_type as i32
}

/// Convert internal MemoryNote to Protobuf MemoryNote
pub fn memory_note_to_proto(note: InternalMemoryNote) -> generated::MemoryNote {
    generated::MemoryNote {
        id: note.id.to_string(),
        namespace: Some(namespace_to_proto(note.namespace)),
        created_at: note.created_at.timestamp() as u64,
        updated_at: note.updated_at.timestamp() as u64,
        content: note.content,
        summary: note.summary,
        keywords: note.keywords,
        tags: note.tags,
        context: note.context,
        memory_type: memory_type_to_proto(note.memory_type),
        importance: note.importance as u32,
        confidence: note.confidence,
        links: note
            .links
            .into_iter()
            .map(|link| generated::MemoryLink {
                target_id: link.target_id.to_string(),
                link_type: link_type_to_proto(link.link_type),
                strength: link.strength,
                reason: link.reason,
                created_at: link.created_at.timestamp() as u64,
                last_traversed_at: link.last_traversed_at.map(|dt| dt.timestamp() as u64),
                user_created: link.user_created,
            })
            .collect(),
        related_files: note.related_files,
        related_entities: note.related_entities,
        access_count: note.access_count as u64,
        last_accessed_at: note.last_accessed_at.timestamp() as u64,
        expires_at: note.expires_at.map(|dt| dt.timestamp() as u64),
        is_archived: note.is_archived,
        superseded_by: note.superseded_by.map(|id| id.to_string()),
        embedding: note.embedding.unwrap_or_default(),
        embedding_model: note.embedding_model,
    }
}
