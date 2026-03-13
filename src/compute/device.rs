use std::collections::HashMap;
use thiserror::Error;
use wgpu::{Adapter, Device, DeviceType, Instance, Queue};

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("No adapters found")]
    NoAdapters,
    #[error("Adapter not found: {0}")]
    AdapterNotFound(String),
    #[error("Failed to request device: {0}")]
    RequestDevice(String),
}

#[derive(Clone, Debug)]
pub struct DeviceHandle {
    pub id: String,
    pub name: String,
    pub vendor: u32,
    pub device_id: u32,
    pub device_type: DeviceType,
    pub index: usize,
    adapter: Adapter,
}

impl DeviceHandle {
    pub async fn connect(&self) -> Result<(Device, Queue), DeviceError> {
        self.adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(|e| DeviceError::RequestDevice(e.to_string()))
    }
}

#[derive(Debug)]
pub struct DeviceRegistry {
    devices: Vec<DeviceHandle>,
    id_to_index: HashMap<String, usize>,
}

impl DeviceRegistry {
    pub fn enumerate() -> Result<Self, DeviceError> {
        let instance = Instance::default();
        
        let adapters = instance.enumerate_adapters(wgpu::Backends::all());

        if adapters.is_empty() {
            return Err(DeviceError::NoAdapters);
        }

        let name_counts: HashMap<String, usize> = adapters
            .iter()
            .map(|a| a.get_info().name.clone())
            .fold(HashMap::new(), |mut acc, name| {
                *acc.entry(name).or_insert(0) += 1;
                acc
            });

        let mut devices = Vec::new();
        let mut id_to_index = HashMap::new();
        let mut name_indices: HashMap<String, usize> = HashMap::new();

        for (idx, adapter) in adapters.into_iter().enumerate() {
            let info = adapter.get_info();
            let base_name = info.name.clone();
            
            let id = if name_counts.get(&base_name).copied().unwrap_or(0) > 1 {
                let sub_index = name_indices.entry(base_name.clone()).or_insert(0);
                let id = format!("{}:{}", base_name, sub_index);
                *sub_index += 1;
                id
            } else {
                base_name.clone()
            };
            
            let handle = DeviceHandle {
                id: id.clone(),
                name: base_name,
                vendor: info.vendor,
                device_id: info.device,
                device_type: info.device_type,
                index: idx,
                adapter,
            };
            
            id_to_index.insert(id.clone(), devices.len());
            devices.push(handle);
        }

        Ok(Self { devices, id_to_index })
    }

    pub fn get(&self, id: &str) -> Option<&DeviceHandle> {
        self.id_to_index.get(id).map(|&idx| &self.devices[idx])
    }

    pub fn devices(&self) -> &[DeviceHandle] {
        &self.devices
    }

    pub fn by_type(&self, device_type: DeviceType) -> Vec<&DeviceHandle> {
        self.devices
            .iter()
            .filter(|d| d.device_type == device_type)
            .collect()
    }

    pub fn discrete_gpus(&self) -> Vec<&DeviceHandle> {
        self.by_type(DeviceType::DiscreteGpu)
    }

    pub fn integrated_gpus(&self) -> Vec<&DeviceHandle> {
        self.by_type(DeviceType::IntegratedGpu)
    }
}