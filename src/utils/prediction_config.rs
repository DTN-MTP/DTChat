use std::collections::HashMap;
use std::sync::{RwLock, Mutex};
use a_sabr::{
    node::Node,
    contact::Contact,
    node_manager::none::NoManagement,
    contact_manager::legacy::evl::EVLManager,
    contact_plan::from_ion_file::IONContactPlan,
    routing::Router,
    routing::aliases::build_generic_router,
    types::{NodeID, Date},
    bundle::Bundle,
    utils::pretty_print
};
use chrono::{Timelike, Utc, DateTime, NaiveDateTime};
use libc::UTIME_NOW;
use std::io;

use crate::utils::socket::Endpoint;

pub struct prediction_config {
    ion_to_node_id : RwLock<HashMap<String,NodeID>>,
    router : Mutex<Box<dyn Router<NoManagement,EVLManager>+ Send + Sync>>,
    cp_start_time : f64,
}

impl prediction_config {
    pub fn new(contact_plan: &str) -> io::Result<Self> {

        println!("RAW contact plan : ");
        println!("{}",contact_plan);

        let (nodes, contacts) = IONContactPlan::parse::<NoManagement, EVLManager>(contact_plan)?;

        let ion_to_node_id = Self::map_node_indices(contact_plan)?;

        // Generate the router
        let router = build_generic_router::<NoManagement, EVLManager>(
            "CgrFirstEndingContactGraph",
            nodes,
            contacts,
            None
        );

        let router: Box<dyn Router<NoManagement, EVLManager> + Send + Sync> =
            unsafe { std::mem::transmute(router) };

        let cp_start_time = Utc::now().timestamp() as f64;

        Ok(prediction_config {
            ion_to_node_id: RwLock::new(ion_to_node_id),
            router : Mutex::new(router),
            cp_start_time
        })
    }

    pub fn get_node_id(&self,ion_id:&str) -> Option<NodeID>{
        self.ion_to_node_id.read().unwrap().get(ion_id).copied()
    }

    pub fn f64_to_utc(timestamp: f64) -> DateTime<Utc> {
        let secs = timestamp.trunc() as i64;
        let nsecs = ((timestamp.fract()) * 1_000_000_000.0).round() as u32;
        let naive = NaiveDateTime::from_timestamp_opt(secs, nsecs)
            .expect("Invalid timestamp");
        DateTime::<Utc>::from_utc(naive, Utc)
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
            expiration: Date::MAX,
        };

        let excluded_nodes = vec![];

        let cp_send_time = Utc::now().timestamp() as f64 - self.cp_start_time;

        let mut router = self.router.lock().unwrap();
        match router.route(bundle.source, &bundle, cp_send_time, &excluded_nodes) {
            Some(routing_output) => {
                println!("Route found from ION {} to ION {}!", source_ion, dest_ion);

                // Only display the last element
                if let Some((_contact_ptr, (_contact, route_stages))) = routing_output.first_hops.iter().last() {
                    if let Some(last_stage) = route_stages.last() {
                        // Create a borrow and use it consistently
                        let last_stage_borrowed = last_stage.borrow();

                        let delay = last_stage_borrowed.at_time;

                        println!("#########################################################");
                        println!("the cp_start_time in UTC is : {:?}", prediction_config::f64_to_utc(self.cp_start_time));
                        println!("cp_start_time is {}", self.cp_start_time);
                        println!("cp_send_time is {}", cp_send_time);
                        println!("delay is {}", delay);
                        println!("returned value is {}", delay + self.cp_start_time);
                        println!("returned value in UTC is {:?}", prediction_config::f64_to_utc(delay + self.cp_start_time));

                        return Ok(delay + self.cp_start_time);
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