use std::collections::HashMap;
use std::sync::RwLock;
use a_sabr::{
    node::Node,
    contact::Contact,
    node_manager::none::NoManagement,
    contact_manager::evl::EVLManager,
    contact_plan::from_ion_file::IONContactPlan,
    routing::{Router, build_generic_router},
    types::NodeID,
    bundle::Bundle
};
use std::io;

pub struct NetworkConfig {
    ion_to_node_id : RwLock<HashMap<String,NodeID>>,
    router : Box<dyn Router<NoManagement,EVLManager>>
}

impl NetworkConfig {
    pub fn new(contact_plan: &str) -> io::Result<Self> {
        let (nodes, contacts) = IONContactPlan::parse::<NoManagement, EVLManager>(contact_plan)?;

        // Initialize the hash map
        let mut ion_to_node_id = HashMap::new();

        for node in &nodes {
            let node_id = node.get_node_id();
            let ion_id = node_id.to_string();

            ion_to_node_id.insert(ion_id, node_id);
        }

        // Generate the router
        let router = build_generic_router::<NoManagement, EVLManager>(
            "CgrFirstEndingMpt", // Use this instead of ContactGraph
            nodes,
            contacts,
            None
        );

        Ok(NetworkConfig {
            ion_to_node_id: RwLock::new(ion_to_node_id),
            router,
        })
    }

    pub fn get_node_id(&self,ion_id:&str) -> Option<NodeID>{
        self.ion_to_node_id.read().unwrap().get(ion_id).copied()
    }

    pub fn get_router(&self) -> &Box<dyn Router<NoManagement,EVLManager>> {
        &self.router
    }

    pub fn route_with_ion_ids(&self, source_ion: &str, dest_ion: &str, message_size: f64) -> io::Result<bool> {
        let source_node_id = self.get_node_id(source_ion).unwrap();
        let dest_node_id = self.get_node_id(dest_ion).unwrap();

        let bundle = Bundle {
            source: source_node_id,
            destinations: vec![dest_node_id],
            priority: 0,
            size: message_size,
            expiration: 10000.0,
        };

        match self.router.route(source_node_id, &bundle, 0.0, &vec![]) {
            Some(_) => {
                println!("✅ Route found from ION {} to ION {}!", source_ion, dest_ion);
                Ok(true)
            }
            None => {
                println!("❌ No route found from ION {} to ION {}!", source_ion, dest_ion);
                Ok(false)
            }
        }
}

}