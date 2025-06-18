use std::collections::HashMap;
use std::sync::{RwLock, Mutex};
use a_sabr::{
    node::Node,
    contact::Contact,
    node_manager::none::NoManagement,
    contact_manager::evl::EVLManager,
    contact_plan::from_ion_file::IONContactPlan,
    routing::Router,
    routing::aliases::build_generic_router,
    types::NodeID,
    bundle::Bundle,
    utils::pretty_print
};
use std::io;

pub struct NetworkConfig {
    ion_to_node_id : RwLock<HashMap<String,NodeID>>,
    router : Mutex<Box<dyn Router<NoManagement,EVLManager>+ Send + Sync>>
}

impl NetworkConfig {
    pub fn new(contact_plan: &str) -> io::Result<Self> {
        let (nodes, contacts) = IONContactPlan::parse::<NoManagement, EVLManager>(contact_plan)?;

        let ion_to_node_id = Self::map_node_indices(contact_plan)?;

        // Generate the router
        let router = build_generic_router::<NoManagement, EVLManager>(
            "CgrFirstEndingMpt", // Use this instead of ContactGraph
            nodes,
            contacts,
            None
        );

        let router: Box<dyn Router<NoManagement, EVLManager> + Send + Sync> =
            unsafe { std::mem::transmute(router) };

        Ok(NetworkConfig {
            ion_to_node_id: RwLock::new(ion_to_node_id),
            router : Mutex::new(router),
        })
    }

    pub fn get_node_id(&self,ion_id:&str) -> Option<NodeID>{
        self.ion_to_node_id.read().unwrap().get(ion_id).copied()
    }


    pub fn map_node_indices(contact_plan: &str) -> io::Result<HashMap<String, NodeID>> {
        let (nodes, _contacts) = IONContactPlan::parse::<NoManagement, EVLManager>(contact_plan)?;
        let node_index_map: HashMap<String, NodeID> = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.get_node_name().to_string(), index as NodeID))
            .collect();
        Ok(node_index_map)
    }


    pub fn route_with_ion_ids(&self, source_ion: &str, dest_ion: &str, message_size: f64) -> io::Result<bool> {

        println!("the data in the hashmap is : {:?}", self.ion_to_node_id.read().unwrap());

        println!("🔍 Looking for route from '{}' to '{}'", source_ion, dest_ion);

        let source_node_id = self.get_node_id(source_ion).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Source ION ID '{}' not found in contact plan", source_ion)
            )
        })?;

        let dest_node_id = self.get_node_id(dest_ion).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Destination ION ID '{}' not found in contact plan", dest_ion)
            )
        })?;

        println!("✅ Found nodes: {} -> {}, {} -> {}", source_ion, source_node_id, dest_ion, dest_node_id);

        let bundle = Bundle {
            source: source_node_id,
            destinations: vec![dest_node_id],
            priority: 0,
            size: message_size,
            expiration: 10000.0,
        };

        let current_time = 0.0;
        let excluded_nodes = vec![];

        let mut router = self.router.lock().unwrap();
        match router.route(bundle.source, &bundle, current_time, &excluded_nodes) {
            Some(routing_output) => {
                println!("Route found from ION {} to ION {}!", source_ion, dest_ion);
                for (_contact_ptr, (contact, route_stages)) in &routing_output.first_hops {
                    let contact_borrowed: std::cell::Ref<'_, Contact<NoManagement, EVLManager>> = contact.as_ref().borrow();
                    println!("First hop: Contact {} -> {} (Start: {}, End: {})",
                        contact_borrowed.info.tx_node,
                        contact_borrowed.info.rx_node,
                        contact_borrowed.info.start,
                        contact_borrowed.info.end
                    );
                    for route_stage in route_stages {
                        pretty_print(route_stage.clone());
                    }
                }
                Ok(true)
            }
            None => {
                println!("No route found from ION {} to ION {}", source_ion, dest_ion);
                Ok(false)
            }
        }
    }

}