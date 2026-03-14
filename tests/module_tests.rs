use alkminer::modules::IncrementModule;
use alkminer::compute::ComputeModule;

mod common;

#[tokio::test]
async fn test_increment_module_setup() {
    let (device, queue) = common::create_test_device().await;
    
    let mut module = IncrementModule::new(64);
    let result = module.setup(&device, &queue).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_increment_module_roundtrip() {
    let (device, queue) = common::create_test_device().await;
    
    let mut module = IncrementModule::new(4);
    module.setup(&device, &queue).await.expect("setup failed");
    
    let input: Vec<u8> = vec![10, 0, 0, 0, 20, 0, 0, 0, 30, 0, 0, 0, 40, 0, 0, 0];
    module.write(&queue, &input).expect("write failed");
    
    module.run(&device, &queue).await.expect("run failed");
    
    let output = module.read_output(&device).await.expect("read failed");
    
    let output_u32: Vec<u32> = output.chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    
    assert_eq!(output_u32, vec![11, 21, 31, 41]);
}

#[tokio::test]
async fn test_increment_module_large() {
    let (device, queue) = common::create_test_device().await;
    
    let count = 256u64;
    let mut module = IncrementModule::new(count);
    module.setup(&device, &queue).await.expect("setup failed");
    
    let input: Vec<u8> = (0..count).flat_map(|i| (i as u32).to_ne_bytes()).collect();
    module.write(&queue, &input).expect("write failed");
    
    module.run(&device, &queue).await.expect("run failed");
    
    let output = module.read_output(&device).await.expect("read failed");
    let output_u32: Vec<u32> = output.chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    
    for i in 0..count {
        assert_eq!(output_u32[i as usize], i as u32 + 1);
    }
}

#[tokio::test]
async fn test_increment_module_destroy() {
    let (device, queue) = common::create_test_device().await;
    
    let mut module = IncrementModule::new(16);
    module.setup(&device, &queue).await.expect("setup failed");
    
    module.destroy();
    
    assert_eq!(module.count(), 16);
}
