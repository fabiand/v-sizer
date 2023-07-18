use sizer::*;

fn main() {
    let node = NodeCapacity {
        resources: Resources {
            memory_bytes: 256 * GI_B,
            cpus: 128
        }
    };

    let c = Cluster {
        schedulable_control_plane: false,
        control_plane_node_count: 3,
        worker_node_count: 3,
        worker_node_capacity: node,
        cpu_over_commit_ratio: 0.1
    };

    println!("Cluster: {:?}", c);

    let estimator = HyperConvergedClusterEstimator{};
    let estimate = estimator.capacity_of(&c);
    println!("Cluster capacity: {:?}", estimate);

    let u1_m = InstanceType {
        name: "u1.medium".to_string(),
        guest: Resources {
            memory_bytes: 4 * GI_B,
            cpus: 8
        },
        consumed: Resources {
            memory_bytes: 200 * MI_B,
            cpus: 1
        },
        overhead: Resources {
            memory_bytes: 0,
            cpus: 0
        }
    };

    let w = Workloads {
        total_count_vm: 100,
        instance_type: u1_m
    };
    println!("Workloads: {:?}", w);
    let required_resources = w.required_resources();
    println!("Workloads requests: {:?}", required_resources);

    println!("Cluster workload capacity: {:?}", estimate.resources.workload - required_resources);
}
