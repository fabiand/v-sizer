use sizer::*;

fn main() {
    let node = Node {
        description: "Worker node".to_owned(),
        resources: Resources {
            memory_bytes: 256 * GI_B,
            cpus: 128
        }
    };

    let c = Cluster {
        description: "".to_owned(),
        schedulable_control_plane: false,
        control_plane_node_count: 3,
        worker_node_count: 3,
        worker_node: node.clone(),
        cpu_over_commit_ratio: 0.1
    };

    println!("Cluster: {}", c);

    let u1_m = InstanceType {
        name: "u1.medium".to_string(),
        guest: Resources {
            memory_bytes: 4 * GI_B,
            cpus: 8
        },
        consumed_by_system: Resources {
            memory_bytes: 200 * MI_B,
            cpus: 1
        },
        reserved_for_overhead: Resources {
            memory_bytes: 0,
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
    println!("Cluster estimate for workload: {}", estimator.capacity_for(&node, &w));
}
