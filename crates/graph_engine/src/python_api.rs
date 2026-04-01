use std::path::Path;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use crate::{
    compute_critical_path_as_json,
    compute_critical_path_from_file,
};

fn map_aoe_error(err: crate::AoeError) -> PyErr {
    match err {
        crate::AoeError::CycleDetected { .. }
        | crate::AoeError::UnknownTask(_)
        | crate::AoeError::ParseError(_) => PyValueError::new_err(err.to_string()),
        crate::AoeError::IoError(_) => PyRuntimeError::new_err(err.to_string()),
    }
}

#[pyfunction]
pub fn critical_path_from_json(json_input: &str) -> PyResult<String> {
    compute_critical_path_as_json(json_input).map_err(map_aoe_error)
}

#[pyfunction]
pub fn critical_path_from_file(path: &str) -> PyResult<String> {
    let result = compute_critical_path_from_file(Path::new(path)).map_err(map_aoe_error)?;
    serde_json::to_string(&result)
        .map_err(|e| PyRuntimeError::new_err(format!("failed to serialize result: {e}")))
}

#[pymodule]
fn graph_engine(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(critical_path_from_json, m)?)?;
    m.add_function(wrap_pyfunction!(critical_path_from_file, m)?)?;
    Ok(())
}
