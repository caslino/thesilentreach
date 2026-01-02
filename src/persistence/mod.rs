use bevy::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

// Data model for a discovery
#[derive(Debug, Clone)]
pub struct Discovery {
    pub system_id: u64,
    pub name: String,
    pub finder: String,
}

#[async_trait]
pub trait DiscoveryRepository: Send + Sync {
    async fn save_discovery(&self, discovery: Discovery) -> Result<(), String>;
    async fn get_system_name(&self, system_id: u64) -> Result<Option<String>, String>;
}

// Mock implementation for now
pub struct MockRepository;

#[async_trait]
impl DiscoveryRepository for MockRepository {
    async fn save_discovery(&self, discovery: Discovery) -> Result<(), String> {
        info!("(Mock) Persisting discovery: {:?}", discovery);
        Ok(())
    }
    
    async fn get_system_name(&self, system_id: u64) -> Result<Option<String>, String> {
        info!("(Mock) Fetching system name for {}", system_id);
        Ok(None)
    }
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        // In a real app we'd insert the repository as a Resource
        // app.insert_resource(Arc::new(MockRepository) as Arc<dyn DiscoveryRepository>);
    }
}
