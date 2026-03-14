use futures::future::BoxFuture;
use thiserror::Error;
use wgpu::{Device, Queue};

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Module not initialized")]
    NotInitialized,
    #[error("Setup failed: {0}")]
    SetupFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

pub trait ComputeModule: Send + Sync {
    fn setup<'a>(
        &'a mut self,
        device: &'a Device,
        queue: &'a Queue,
    ) -> BoxFuture<'a, Result<(), ModuleError>>;
    fn run<'a>(
        &'a mut self,
        device: &'a Device,
        queue: &'a Queue,
    ) -> BoxFuture<'a, Result<(), ModuleError>>;
    fn destroy(&mut self);
}
