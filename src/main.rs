use sizer::*;
use serde_json;
use std::fs;
use std::collections::HashMap;
use clap::{Parser, Subcommand};
use glob::glob;


#[cfg(test)]
mod test {
    use sizer::*;
    use byte_unit::Byte;

    #[test]
    fn de_serialize_node() {
        assert!(true);
    }

}

/// Perform estimations around OpenShift Virtualization.
/// Provide a target workload, and estimate the required cluster size
/// Provide a cluster size, and estimate it's workload capacity
#[derive(Parser)]
struct SizerCli {
    #[command(subcommand)]
    command: SizerCommands,

    /// Path to the nodes.d dir with all node definitions
    #[arg(short, long, default_value = "data/nodes.d/")]
    nodes_path: String,
}

#[derive(Subcommand)]
enum SizerCommands {
    /// Estimate the required cluster for a given workload
    CapacityOf(CapacityArgs),
    /// Estimate the capacity of a given cluster
    FootprintOf(FootprintArgs),
    RequiredClusterFor(RequiredClusterForArgs)
}

#[derive(Parser)]
struct CapacityArgs {
    /// Name of a node definition defined in nodes.d
    #[arg(long)]
    node: Option<String>,

    /// File with the node definition
    #[arg(long, default_value = "data/node-simple.json")]
    node_file: String,

    /// File with the cluster definition
    #[arg(short, long, default_value = "data/cluster-simple.json")]
    cluster_file: String,
}

#[derive(Parser)]
struct FootprintArgs {
    /// Determine the required cluster for a given workload and node type
    /// File with the instanceType definition
    #[arg(short, long, default_value = "data/u1medium.json")]
    instancetype_file: String,

    /// File with the workload definition
    #[arg(short, long, default_value = "data/workload-simple.json")]
    workload_file: String,
}

#[derive(Parser)]
struct RequiredClusterForArgs {
    #[arg(short, long, default_value = "data/clustertopology-simple.json")]
    clustertopology_file: String,

    #[arg(short, long, default_value = "data/workload-simple.json")]
    workload_file: String,
}


fn load_from_file<T: for <'a> serde::de::Deserialize<'a>>(f: &String) -> Result<T, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(f)?;
    let obj = serde_json::from_str(&data)?;
    Ok(obj)
}

struct NodeRegistry {
    nodes: HashMap<String, Node>
}

impl NodeRegistry {
    fn create_from_path(path: String) -> NodeRegistry {
        let mut nodes = HashMap::new();

        for entry in glob(&(path.to_string() + "/*.json")).unwrap() {
            match entry {
                Ok(entryfn) => {
                    let fnn = entryfn.clone().into_os_string().into_string().unwrap();
                    let _ = &nodes.insert(entryfn.file_stem().unwrap().to_os_string().into_string().unwrap().to_owned(),
                                  load_from_file(&fnn).unwrap());
                    },
                Err(_) => panic!()
            }
        }

        NodeRegistry {
            nodes: nodes
        }
    }

    fn print(&self) {
        println!("Node registry");
        for (key, val) in self.nodes.iter() {
            println!("key: {key} val: {val}");
        }
    }
}

fn main() {
    let args = SizerCli::parse();

    let node_registry = NodeRegistry::create_from_path(args.nodes_path);
    //node_registry.print();

    match &args.command {
        SizerCommands::CapacityOf(cmd) => {
            println!("aaaan: {:?}", &cmd.node.as_ref());
            if let Some(node_name) = &cmd.node {
                println!("Using node form index");
                node_registry.nodes.get(&node_name.to_owned());
                // do somethin with it
            }
            if !cmd.cluster_file.is_empty() {
                let c: Cluster = load_from_file(&cmd.cluster_file).unwrap();
                println!("Cluster: {}", c);
                // Let's estimate the capacity of the cluster
                println!("Estimated cluster capacity: {}", c.resources());
            } else {
                // FIXME print help, how!?
                todo!()
            }
        },
        SizerCommands::FootprintOf(cmd) => {
            if !cmd.workload_file.is_empty() {
                let workload: Workloads = load_from_file(&cmd.workload_file).unwrap();
                println!("Workloads: {}", workload);
                println!("Workload resource footprint: {}", workload.required_resources());
            } else {
                todo!()
            }
        },
        SizerCommands::RequiredClusterFor(cmd) => {
            let topology: ClusterTopology = load_from_file(&cmd.clustertopology_file).unwrap();
            let workload: Workloads = load_from_file(&cmd.workload_file).unwrap();
            println!("ClusterTopology: {}", topology);
            println!("Workloads: {}", workload);
            println!("Workload resource footprint: {}", workload.required_resources());

            let reasoned_cluster = Cluster::for_topology_and_workload(topology, workload);
            let reasons = reasoned_cluster.reasons;
            let cluster = reasoned_cluster.result;
            println!("Cluster: {}", &cluster);
            println!("Cluster capacity: {}", &cluster.resources());
            println!("Reasoning:\n- {}", &reasons.join("\n- "));
        }
    }
/*
    // ...  do these fit into the cluster?
    println!("Estimated workload capacity: {}", estimate.resources.available_to_workloads);

    // Then, how many do fit into the cluster?
    println!("Workload fit into estimate? {}", w.can_fit_into(&estimate.resources));
    let (cap, reas) = u1_m.how_many_fit_into(&estimate.resources);
    println!("Workload how many fit into estimate? {} constrained by {}", cap, reas);

    */
}
