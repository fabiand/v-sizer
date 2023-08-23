use sizer::*;
use serde_json;
use std::fs;
use clap::{Parser, Subcommand};


#[cfg(test)]
mod test {
    use sizer::*;
    use byte_unit::Byte;

    #[test]
    fn de_serialize_node() {
        let c_data = r#"{
            "description": "",
            "schedulable_control_plane": false,
            "control_plane_node_count": 3,
            "worker_node_count": 3,
            "worker_node": {
                "description": "Worker node",
                "resources": {
                    "memory": "256 GiB",
                    "cpus": 128
                }
            },
            "cpu_over_commit_ratio": 0.1
        }"#;
        let _c: Cluster = serde_json::from_str(c_data).unwrap();

        // FIXME assert
    }

    #[test]
    fn de_serialize_instancetype() {
        let _u1_m = InstanceType {
            name: "u1.medium".to_string(),
            guest: Resources {
                memory: Byte::from_str("4 GiB").unwrap(),
                cpus: 8
            },
            consumed_by_system: Resources {
                memory: Byte::from_str("200 MiB").unwrap(),
                cpus: 1
            },
            reserved_for_overhead: Resources {
                memory: Byte::from_str("0").unwrap(),
                cpus: 0
            }
        };

        // FIXME assert
    }

}

/// Perform estimations around OpenShift Virtualization.
/// Provide a target workload, and estimate the required cluster size
/// Provide a cluster size, and estimate it's workload capacity
#[derive(Parser)]
struct SizerCli {
    #[command(subcommand)]
    command: SizerCommands
}

#[derive(Subcommand)]
enum SizerCommands {
    /// Estimate the required cluster for a given workload
    EstimateClusterFor(EstimateClusterForArgs),
    /// Estimate the capacity of a given cluster
    EstimateCapacityOf(EstimateCapacityOfArgs)
}

#[derive(Parser)]
struct EstimateClusterForArgs {
    /// File with the instanceType definition
    #[arg(short, long, default_value = "data/u1medium.json")]
    instancetype_file: String,

    /// File with the workload definition
    #[arg(short, long, default_value = "data/workload-simple.json")]
    workload_file: String,

    /// File with the node definition
    #[arg(short, long, default_value = "data/node-simple.json")]
    node_file: String,
}

#[derive(Parser)]
struct EstimateCapacityOfArgs {
    /// File with the cluster definition
    #[arg(short, long, default_value = "data/cluster-simple.json")]
    cluster_file: String,
}

fn load_from_file<T: for <'a> serde::de::Deserialize<'a>>(f: &String) -> Result<T, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(f)?;

    let obj = serde_json::from_str(&data)?;

    Ok(obj)
}

fn main() {
    let args = SizerCli::parse();

    let estimator = HyperConvergedClusterEstimator{};

    match &args.command {
        SizerCommands::EstimateClusterFor(cmd) => {
            let workload: Workloads = load_from_file(&cmd.workload_file).unwrap();
            println!("Workloads: {}", workload);

            let node: Node = load_from_file(&cmd.node_file).unwrap();
            println!("Node: {}", node);

            // What cluster would I eventually need for the workloads?
            println!("Cluster estimate for workload: {}", estimator.capacity_for(&node, &workload));
        },
        SizerCommands::EstimateCapacityOf(cmd) => {
            let c: Cluster = load_from_file(&cmd.cluster_file).unwrap();
            println!("Cluster: {}", c);

            // Let's estimate the capacity of the cluster
            let estimate = estimator.capacity_of(&c);
            println!("Estimated cluster capacity: {}", estimate);
        }
    }
/*
    // Ok, let's assume these workloads
    let w = Workloads {
        vm_count: 100,
        instance_type: &u1_m
    };
    println!("Workloads: {}", w);

    // ...  do these fit into the cluster?
    println!("Estimated workload capacity: {}", estimate.resources.available_to_workloads);
    println!("Avail - req: {}",  &estimate.resources.available_to_workloads - &w.required_resources());

    // Then, how many do fit into the cluster?
    println!("Workload fit into estimate? {}", w.can_fit_into(&estimate.resources));
    let (cap, reas) = u1_m.how_many_fit_into(&estimate.resources);
    println!("Workload how many fit into estimate? {} constrained by {}", cap, reas);

    */
}
