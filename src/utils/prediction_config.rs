use a_sabr::{
    bundle::Bundle,
    contact_manager::legacy::evl::EVLManager,
    contact_plan::from_ion_file::IONContactPlan,
    node_manager::none::NoManagement,
    routing::aliases::build_generic_router,
    routing::Router,
    types::{Date, NodeID},
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::io;
use std::sync::{Mutex, RwLock};

use crate::network::Endpoint;

pub struct PredictionConfig {
    ion_to_node_id: RwLock<HashMap<String, NodeID>>,
    router: Mutex<Box<dyn Router<NoManagement, EVLManager> + Send + Sync>>,
    cp_start_time: f64,
}

impl PredictionConfig {
    pub fn new(contact_plan: &str) -> io::Result<Self> {
        println!("RAW contact plan : ");
        println!("{contact_plan}");

        let (nodes, contacts) = IONContactPlan::parse::<NoManagement, EVLManager>(contact_plan)?;

        let ion_to_node_id = Self::map_node_indices(contact_plan)?;

        // Generate the router
        let router = build_generic_router::<NoManagement, EVLManager>(
            "CgrFirstEndingContactGraph",
            nodes,
            contacts,
            None,
        );

        let router: Box<dyn Router<NoManagement, EVLManager> + Send + Sync> =
            unsafe { std::mem::transmute(router) };

        let cp_start_time = Utc::now().timestamp() as f64;

        Ok(PredictionConfig {
            ion_to_node_id: RwLock::new(ion_to_node_id),
            router: Mutex::new(router),
            cp_start_time,
        })
    }

    pub fn get_node_id(&self, ion_id: &str) -> Option<NodeID> {
        self.ion_to_node_id.read().unwrap().get(ion_id).copied()
    }

    pub fn f64_to_utc(timestamp: f64) -> DateTime<Utc> {
        let secs = timestamp.trunc() as i64;
        let nsecs = ((timestamp.fract()) * 1_000_000_000.0).round() as u32;
        let naive = DateTime::from_timestamp(secs, nsecs).expect("Invalid timestamp");
        DateTime::from_naive_utc_and_offset(naive.naive_utc(), Utc)
    }

    pub fn extract_ion_node_from_endpoint(endpoint: &Endpoint) -> Option<String> {
        match endpoint {
            Endpoint::Bp(bp_address) => {
                // Handle ipn: format (e.g., "ipn:10.1" -> "10")
                if let Some(after_ipn) = bp_address.strip_prefix("ipn:") {
                    if let Some(dot_pos) = after_ipn.find('.') {
                        return Some(after_ipn[..dot_pos].to_string());
                    }
                }
                if bp_address.chars().all(|c| c.is_ascii_digit()) {
                    return Some(bp_address.clone());
                }
                Some(bp_address.clone())
            }
            Endpoint::Udp(_) | Endpoint::Tcp(_) => None,
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

    pub fn predict(&self, source_ion: &str, dest_ion: &str, message_size: f64) -> io::Result<Date> {
        let source_node_id = self.get_node_id(source_ion).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Source ION ID '{source_ion}' not found in contact plan"),
            )
        })?;

        let dest_node_id = self.get_node_id(dest_ion).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Destination ION ID '{dest_ion}' not found in contact plan"),
            )
        })?;

        let bundle = Bundle {
            source: source_node_id,
            destinations: vec![dest_node_id],
            priority: 0,
            size: message_size,
            expiration: Date::MAX,
        };

        let excluded_nodes = vec![];

        let cp_send_time = Utc::now().timestamp() as f64 - self.cp_start_time;

        let mut router = self.router.lock().unwrap();
        match router.route(bundle.source, &bundle, cp_send_time, &excluded_nodes) {
            Some(routing_output) => {
                println!("Route found from ION {source_ion} to ION {dest_ion}!");
                // Only display the last element
                if let Some((_contact_ptr, (_contact, route_stages))) =
                    routing_output.first_hops.iter().last()
                {
                    if let Some(last_stage) = route_stages.last() {
                        // Create a borrow and use it consistently
                        let last_stage_borrowed = last_stage.borrow();

                        let delay = last_stage_borrowed.at_time;

                        println!("#########################################################");
                        println!(
                            "the cp_start_time in UTC is : {:?}",
                            PredictionConfig::f64_to_utc(self.cp_start_time)
                        );
                        println!(
                            "the delay in UTC is : {:?}",
                            PredictionConfig::f64_to_utc(delay + self.cp_start_time)
                        );
                        println!("cp_send_time is {cp_send_time}");
                        println!("the delay in seconds is : {delay}");

                        return Ok(delay + self.cp_start_time);
                    }
                }
                Err(io::Error::other(
                    "Route found but no route stages available",
                ))
            }
            None => {
                println!("No route found from ION {source_ion} to ION {dest_ion}");
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("No route found from ION {source_ion} to ION {dest_ion}"),
                ))
            }
        }
    }
}
