use sizer::*;
use serde_json;
use std::fs;
use byte_unit::Byte;
use clap::Parser;


#[cfg(test)]
mod test {
    use sizer::*;

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
}

#[derive(Parser)]
struct Args {
    // File with the cluster definition
    #[arg(short, long, default_value = "data/cluster-simple.json")]
    cluster_file: String
}

fn main() {
    let args = Args::parse();

    let c_data = fs::read_to_string(args.cluster_file)
        .expect("Unable to read file");
    let c: Cluster = serde_json::from_str(&c_data)
        .expect("JSON does not have the correct format");

    println!("Cluster: {}", c);

    let u1_m = InstanceType {
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

    // Let's estimate the capacity of the cluster
    let estimator = HyperConvergedClusterEstimator{};
    let estimate = estimator.capacity_of(&c);
    println!("Estimated cluster capacity: {}", estimate);

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

    // And: What cluster would I eventually need for the workloads?
    println!("Cluster estimate for workload: {}", estimator.capacity_for(&c.worker_node, &w));
}
