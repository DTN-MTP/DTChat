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
    types::{NodeID, Date},
    bundle::Bundle,
    utils::pretty_print
};
use std::io;

use crate::utils::socket::Endpoint;

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
            "CgrFirstEndingContactGraph", // Use this instead of ContactGraph
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


    pub fn extract_ion_node_from_endpoint(endpoint: &Endpoint) -> Option<String> {
        match endpoint {
            Endpoint::Bp(bp_address) => {
                // Handle ipn: format (e.g., "ipn:10.1" -> "10")
                if bp_address.starts_with("ipn:") {
                    let after_ipn = &bp_address[4..]; // Remove "ipn:" prefix
                    if let Some(dot_pos) = after_ipn.find('.') {
                        return Some(after_ipn[..dot_pos].to_string());
                    } else {
                        // If no dot, return the whole number part
                        return Some(after_ipn.to_string());
                    }
                }
                if bp_address.chars().all(|c| c.is_ascii_digit()) {
                    return Some(bp_address.clone());
                }
                Some(bp_address.clone())
            }
            Endpoint::Udp(_) | Endpoint::Tcp(_) => {
                None
            }
        }
    }

    pub fn test_endpoint(&self, endpoint: &Endpoint) -> bool {
        if let Some(ion_id) = Self::extract_ion_node_from_endpoint(endpoint) {
            self.ion_to_node_id.read().unwrap().contains_key(&ion_id)
        } else {
            false
        }
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


    pub fn route_with_ion_ids(&self, source_ion: &str, dest_ion: &str, message_size: f64) -> io::Result<Date> {

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

                // Only display the last element
                if let Some((_contact_ptr, (_contact, route_stages))) = routing_output.first_hops.iter().last() {
                    if let Some(last_stage) = route_stages.last() {
                        // Extract the time value
                        let time_value = last_stage.borrow().at_time;
                        return Ok(time_value);
                    }
                }
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Route found but no route stages available"
                ))
            }
            None => {
                println!("No route found from ION {} to ION {}", source_ion, dest_ion);
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("No route found from ION {} to ION {}", source_ion, dest_ion)
                ))
            }
        }
    }
}