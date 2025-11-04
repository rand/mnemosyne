//! PyO3 bridge for DSPy-based agent intelligence
//!
//! This module provides a Rust interface to Python DSPy modules for systematic
//! prompt optimization and LLM interactions. It enables:
//!
//! - Type-safe signatures for LLM calls
//! - Automatic prompt optimization via teleprompters
//! - Multi-agent orchestration with GEPA
//! - Semantic analysis for Tier 3 highlighting
//!
//! # Architecture
//!
//! ```text
//! Rust (Ractor Actors) → PyO3 Bridge → Python DSPy → LLM Providers
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
//! use std::collections::HashMap;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize bridge
//! let bridge = DSpyBridge::new()?;
//!
//! // Call agent module
//! let mut inputs = HashMap::new();
//! inputs.insert("user_intent".to_string(), serde_json::json!("Implement user auth"));
//!
//! let result = bridge.call_agent_module("reviewer", inputs).await?;
//! # Ok(())
//! # }
//! ```

// Guard entire module behind python feature flag to support builds without PyO3
#[cfg(feature = "python")]
use crate::error::{MnemosyneError, Result};
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};
#[cfg(feature = "python")]
use serde_json::Value;
#[cfg(feature = "python")]
use std::collections::HashMap;
#[cfg(feature = "python")]
use std::sync::Arc;
#[cfg(feature = "python")]
use tokio::sync::Mutex;
#[cfg(feature = "python")]
use tracing::{debug, error, info, warn};

/// PyO3 bridge to DSPy agent modules
///
/// Manages the Python interpreter and provides async-friendly interface
/// to DSPy modules. Thread-safe via Arc<Mutex<>> for GIL management.
#[cfg(feature = "python")]
#[derive(Clone)]
pub struct DSpyBridge {
    /// Python DSPy service instance (holds GIL when accessed)
    service: Arc<Mutex<Py<PyAny>>>,
}

#[cfg(feature = "python")]
impl DSpyBridge {
    /// Create a new DSPy bridge
    ///
    /// Initializes Python interpreter and imports DSPy service module.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Python interpreter initialization fails
    /// - DSPy module import fails
    /// - DSpyService instantiation fails
    pub fn new() -> Result<Self> {
        Python::with_gil(|py| {
            // Import the DSPy service module
            let dspy_service_mod = py
                .import_bound("mnemosyne.orchestration.dspy_service")
                .map_err(|e| {
                    error!("Failed to import DSPy service module: {}", e);
                    MnemosyneError::Other(format!("DSPy service import failed: {}", e))
                })?;

            // Get DSpyService class
            let service_class = dspy_service_mod.getattr("DSpyService").map_err(|e| {
                error!("Failed to get DSpyService class: {}", e);
                MnemosyneError::Other(format!("DSpyService class not found: {}", e))
            })?;

            // Instantiate service
            let service = service_class.call0().map_err(|e| {
                error!("Failed to instantiate DSpyService: {}", e);
                MnemosyneError::Other(format!("DSpyService instantiation failed: {}", e))
            })?;

            info!("DSPy bridge initialized successfully");

            Ok(Self {
                service: Arc::new(Mutex::new(service.unbind().into())),
            })
        })
    }

    /// Call an agent DSPy module with given inputs
    ///
    /// # Arguments
    ///
    /// * `agent_name` - Name of agent ("orchestrator", "optimizer", "reviewer", "executor")
    /// * `inputs` - Input parameters as JSON values
    ///
    /// # Returns
    ///
    /// HashMap of output field names to JSON values extracted from DSPy prediction
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Agent module not found
    /// - Input conversion fails
    /// - DSPy module call fails
    /// - Output extraction fails
    pub async fn call_agent_module(
        &self,
        agent_name: &str,
        inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let agent_name = agent_name.to_string();
        debug!("Calling DSPy module for agent: {}", agent_name);

        // Acquire lock and call Python (blocking, so run in spawn_blocking)
        let service = self.service.clone();
        let agent_name_clone = agent_name.clone();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                // Get agent module
                let module = service_ref
                    .call_method1("get_agent_module", (&agent_name_clone,))
                    .map_err(|e| {
                        error!("Failed to get agent module '{}': {}", agent_name_clone, e);
                        MnemosyneError::Other(format!("Agent module '{}' not found: {}", agent_name_clone, e))
                    })?;

                // Convert inputs to Python dict
                let py_inputs = PyDict::new_bound(py);
                for (key, value) in &inputs {
                    let py_value = json_to_python(py, value)?;
                    py_inputs.set_item(key, py_value).map_err(|e| {
                        MnemosyneError::Other(format!("Failed to set input '{}': {}", key, e))
                    })?;
                }

                // Call module with inputs (unpacked as kwargs)
                let prediction = module.call((), Some(&py_inputs)).map_err(|e| {
                    error!("DSPy module call failed for '{}': {}", agent_name_clone, e);
                    MnemosyneError::Other(format!("DSPy module call failed: {}", e))
                })?;

                // Extract outputs from prediction
                extract_dspy_outputs(py, &prediction)
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        debug!(
            "DSPy module call for '{}' returned {} outputs",
            agent_name,
            result.len()
        );

        Ok(result)
    }

    /// Get list of available agent modules
    ///
    /// # Returns
    ///
    /// Vector of agent names that have DSPy modules registered
    pub async fn list_agent_modules(&self) -> Result<Vec<String>> {
        let service = self.service.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                let modules = service_ref
                    .call_method0("list_modules")
                    .map_err(|e| {
                        error!("Failed to list agent modules: {}", e);
                        MnemosyneError::Other(format!("Failed to list modules: {}", e))
                    })?;

                // Convert Python list to Vec<String>
                let py_list: &Bound<PyList> = modules.downcast().map_err(|e| {
                    MnemosyneError::Other(format!("Module list is not a list: {}", e))
                })?;

                let mut result = Vec::new();
                for item in py_list.iter() {
                    let name: String = item.extract().map_err(|e| {
                        MnemosyneError::Other(format!("Module name is not a string: {}", e))
                    })?;
                    result.push(name);
                }

                Ok(result)
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Async execution failed: {}", e)))?
    }

    /// Reload all DSPy modules (useful for development)
    ///
    /// Forces reimport of Python modules to pick up code changes without restart.
    pub async fn reload_modules(&self) -> Result<()> {
        let service = self.service.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                service_ref.call_method0("reload_modules").map_err(|e| {
                    error!("Failed to reload DSPy modules: {}", e);
                    MnemosyneError::Other(format!("Module reload failed: {}", e))
                })?;

                info!("DSPy modules reloaded successfully");
                Ok(())
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Async execution failed: {}", e)))?
    }
}

/// Convert serde_json::Value to Python object
#[cfg(feature = "python")]
fn json_to_python(py: Python, value: &Value) -> Result<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Err(MnemosyneError::Other("Invalid number".to_string()))
            }
        }
        Value::String(s) => Ok(s.to_object(py)),
        Value::Array(arr) => {
            let py_list = PyList::empty_bound(py);
            for item in arr {
                let py_item = json_to_python(py, item)?;
                py_list.append(py_item).map_err(|e| {
                    MnemosyneError::Other(format!("Failed to append to list: {}", e))
                })?;
            }
            Ok(py_list.to_object(py))
        }
        Value::Object(obj) => {
            let py_dict = PyDict::new_bound(py);
            for (key, val) in obj {
                let py_val = json_to_python(py, val)?;
                py_dict.set_item(key, py_val).map_err(|e| {
                    MnemosyneError::Other(format!("Failed to set dict item: {}", e))
                })?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

/// Extract outputs from DSPy Prediction object
#[cfg(feature = "python")]
fn extract_dspy_outputs(_py: Python, prediction: &Bound<PyAny>) -> Result<HashMap<String, Value>> {
    let mut outputs = HashMap::new();

    // Get all attributes of the prediction object
    let dir_list = prediction.dir();

    for attr_name in dir_list.iter() {
        let name: String = attr_name.extract().map_err(|e| {
            MnemosyneError::Other(format!("Attribute name is not a string: {}", e))
        })?;

        // Skip private attributes and methods
        if name.starts_with('_') || name == "forward" || name == "dump_state" || name == "load_state" {
            continue;
        }

        // Try to get attribute value
        if let Ok(value) = prediction.getattr(name.as_str()) {
            // Convert to JSON
            if let Ok(json_value) = python_to_json(&value) {
                outputs.insert(name, json_value);
            } else {
                warn!("Failed to convert attribute '{}' to JSON, skipping", name);
            }
        }
    }

    Ok(outputs)
}

/// Convert Python object to serde_json::Value
#[cfg(feature = "python")]
fn python_to_json(obj: &Bound<PyAny>) -> Result<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            Ok(Value::Number(n))
        } else {
            Err(MnemosyneError::Other("Invalid float".to_string()))
        }
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(py_list) = obj.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in py_list.iter() {
            arr.push(python_to_json(&item)?);
        }
        Ok(Value::Array(arr))
    } else if let Ok(py_dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in py_dict.iter() {
            let key_str: String = key.extract().map_err(|e| {
                MnemosyneError::Other(format!("Dict key is not a string: {}", e))
            })?;
            map.insert(key_str, python_to_json(&value)?);
        }
        Ok(Value::Object(map))
    } else {
        // Try to convert to string as fallback
        if let Ok(s) = obj.str() {
            if let Ok(s_str) = s.extract::<String>() {
                Ok(Value::String(s_str))
            } else {
                Err(MnemosyneError::Other("Cannot convert to JSON".to_string()))
            }
        } else {
            Err(MnemosyneError::Other("Cannot convert to JSON".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_python_conversions() {
        Python::with_gil(|py| {
            // Test null
            let null = json_to_python(py, &Value::Null).unwrap();
            assert!(null.is_none(py));

            // Test bool
            let bool_val = json_to_python(py, &Value::Bool(true)).unwrap();
            assert_eq!(bool_val.extract::<bool>(py).unwrap(), true);

            // Test number
            let num_val = json_to_python(py, &Value::Number(42.into())).unwrap();
            assert_eq!(num_val.extract::<i64>(py).unwrap(), 42);

            // Test string
            let str_val = json_to_python(py, &Value::String("test".to_string())).unwrap();
            assert_eq!(str_val.extract::<String>(py).unwrap(), "test");

            // Test array
            let arr_val = json_to_python(
                py,
                &Value::Array(vec![Value::Number(1.into()), Value::Number(2.into())]),
            )
            .unwrap();
            let py_list: &Bound<PyList> = arr_val.downcast_bound(py).unwrap();
            assert_eq!(py_list.len(), 2);

            // Test object
            let mut obj = serde_json::Map::new();
            obj.insert("key".to_string(), Value::String("value".to_string()));
            let obj_val = json_to_python(py, &Value::Object(obj)).unwrap();
            let py_dict: &Bound<PyDict> = obj_val.downcast_bound(py).unwrap();
            assert_eq!(py_dict.len(), 1);
        });
    }

    #[test]
    fn test_python_to_json_conversions() {
        Python::with_gil(|py| {
            // Test null
            let null = python_to_json(&py.None().into_bound(py)).unwrap();
            assert_eq!(null, Value::Null);

            // Test bool
            let bool_val = python_to_json(&true.to_object(py).into_bound(py)).unwrap();
            assert_eq!(bool_val, Value::Bool(true));

            // Test number
            let num_val = python_to_json(&42_i64.to_object(py).into_bound(py)).unwrap();
            assert_eq!(num_val, Value::Number(42.into()));

            // Test string
            let str_val = python_to_json(&"test".to_object(py).into_bound(py)).unwrap();
            assert_eq!(str_val, Value::String("test".to_string()));
        });
    }

    #[test]
    fn test_json_to_python_nested_structures() {
        Python::with_gil(|py| {
            // Test nested object
            let mut inner_obj = serde_json::Map::new();
            inner_obj.insert("nested_key".to_string(), Value::String("nested_value".to_string()));

            let mut outer_obj = serde_json::Map::new();
            outer_obj.insert("inner".to_string(), Value::Object(inner_obj));
            outer_obj.insert("number".to_string(), Value::Number(42.into()));

            let nested_val = json_to_python(py, &Value::Object(outer_obj)).unwrap();
            let py_dict: &Bound<PyDict> = nested_val.downcast_bound(py).unwrap();

            // Verify outer dict has both keys
            assert!(py_dict.contains("inner").unwrap());
            assert!(py_dict.contains("number").unwrap());

            // Verify nested structure
            let inner_dict = py_dict.get_item("inner").unwrap().unwrap();
            let inner_py_dict: &Bound<PyDict> = inner_dict.downcast().unwrap();
            assert!(inner_py_dict.contains("nested_key").unwrap());
        });
    }

    #[test]
    fn test_python_to_json_nested_structures() {
        Python::with_gil(|py| {
            // Create nested Python dict
            let outer_dict = PyDict::new_bound(py);
            let inner_dict = PyDict::new_bound(py);

            inner_dict.set_item("nested_key", "nested_value").unwrap();
            outer_dict.set_item("inner", inner_dict).unwrap();
            outer_dict.set_item("number", 42).unwrap();

            let json_val = python_to_json(&outer_dict).unwrap();

            // Verify JSON structure
            assert!(json_val.is_object());
            let obj = json_val.as_object().unwrap();
            assert_eq!(obj.len(), 2);
            assert!(obj.contains_key("inner"));
            assert!(obj.contains_key("number"));

            // Verify nested object
            let inner_json = obj.get("inner").unwrap();
            assert!(inner_json.is_object());
            let inner_obj = inner_json.as_object().unwrap();
            assert_eq!(inner_obj.get("nested_key").unwrap().as_str().unwrap(), "nested_value");
        });
    }

    #[test]
    fn test_json_to_python_array_conversion() {
        Python::with_gil(|py| {
            let json_array = Value::Array(vec![
                Value::String("item1".to_string()),
                Value::Number(2.into()),
                Value::Bool(true),
            ]);

            let py_list = json_to_python(py, &json_array).unwrap();
            let py_list_bound: &Bound<PyList> = py_list.downcast_bound(py).unwrap();

            assert_eq!(py_list_bound.len(), 3);
            assert_eq!(py_list_bound.get_item(0).unwrap().extract::<String>().unwrap(), "item1");
            assert_eq!(py_list_bound.get_item(1).unwrap().extract::<i64>().unwrap(), 2);
            assert_eq!(py_list_bound.get_item(2).unwrap().extract::<bool>().unwrap(), true);
        });
    }

    #[test]
    fn test_python_to_json_array_conversion() {
        Python::with_gil(|py| {
            let py_list = PyList::new_bound(py, &["item1", "item2", "item3"]);

            let json_val = python_to_json(&py_list).unwrap();

            assert!(json_val.is_array());
            let arr = json_val.as_array().unwrap();
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_str().unwrap(), "item1");
            assert_eq!(arr[1].as_str().unwrap(), "item2");
            assert_eq!(arr[2].as_str().unwrap(), "item3");
        });
    }

    #[test]
    fn test_json_numbers() {
        Python::with_gil(|py| {
            // Test integer
            let int_val = json_to_python(py, &Value::Number(42.into())).unwrap();
            assert_eq!(int_val.extract::<i64>(py).unwrap(), 42);

            // Test float
            let float_json = serde_json::Number::from_f64(3.14).unwrap();
            let float_val = json_to_python(py, &Value::Number(float_json)).unwrap();
            let extracted_float = float_val.extract::<f64>(py).unwrap();
            assert!((extracted_float - 3.14).abs() < 0.001);
        });
    }

    #[test]
    fn test_dspy_bridge_clone() {
        // Test that DSpyBridge can be cloned (important for Arc sharing)
        let bridge = DSpyBridge::new().unwrap();
        let bridge_clone = bridge.clone();

        // Both should work independently
        // This validates the Clone implementation works correctly
        drop(bridge);
        drop(bridge_clone);
    }
}
