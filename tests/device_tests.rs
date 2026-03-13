use alkminer::compute::DeviceRegistry;

mod common;

#[test]
fn test_mock_registry_creates_correct_count() {
    let registry = DeviceRegistry::mock(4);
    assert_eq!(registry.devices().len(), 4);
}

#[test]
fn test_mock_registry_ids() {
    let registry = DeviceRegistry::mock(3);
    assert!(registry.get("MockGPU:0").is_some());
    assert!(registry.get("MockGPU:1").is_some());
    assert!(registry.get("MockGPU:2").is_some());
    assert!(registry.get("MockGPU:3").is_none());
}

#[test]
fn test_mock_device_is_mock() {
    let registry = DeviceRegistry::mock(2);
    let device = registry.get("MockGPU:0").unwrap();
    assert!(device.is_mock());
}

#[test]
fn test_mock_device_cannot_connect() {
    let registry = DeviceRegistry::mock(1);
    let device = registry.get("MockGPU:0").unwrap().clone();
    
    common::run_test(|| async move {
        let result = device.connect().await;
        assert!(result.is_err());
    });
}

#[test]
fn test_mock_device_properties() {
    let registry = DeviceRegistry::mock(2);
    let device = registry.get("MockGPU:1").unwrap();
    
    assert_eq!(device.id, "MockGPU:1");
    assert_eq!(device.name, "MockGPU");
    assert_eq!(device.index, 1);
    assert_eq!(device.device_type, wgpu::DeviceType::Cpu);
}

#[test]
fn test_enumerate_finds_adapters() {
    let result = DeviceRegistry::enumerate();
    assert!(result.is_ok());
    
    let registry = result.unwrap();
    assert!(!registry.devices().is_empty());
}

#[tokio::test]
async fn test_real_device_can_connect() {
    let registry = DeviceRegistry::enumerate().expect("Failed to enumerate");
    let device = registry.devices().first().unwrap();
    
    if !device.is_mock() {
        let result = device.connect().await;
        assert!(result.is_ok());
    }
}
