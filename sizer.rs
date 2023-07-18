use std::ops;
use std::cmp::Ordering;

const MI_B: i64 = 1024 * 1024;
const GI_B: i64 = MI_B * 1024;
 
#[derive(Clone, Copy, Debug)]
struct Resources {
    memory_bytes: i64,
    cpus: i64
}

impl ops::Add<Resources> for Resources {
    type Output = Self;
    fn add(self, _rhs: Self) -> Self {
        Self { memory_bytes: self.memory_bytes + _rhs.memory_bytes, cpus: self.cpus + _rhs.cpus }
    }
}

impl ops::Sub<Resources> for Resources {
    type Output = Self;
    fn sub(self, _rhs: Self) -> Self {
        Self { memory_bytes: self.memory_bytes - _rhs.memory_bytes, cpus: self.cpus - _rhs.cpus }
    }
}

#[derive(Debug)]
struct Workloads {
    total_count_vm: i64,
    instance_type: InstanceType,
}

#[derive(Debug)]
struct InstanceType {
    name: String,
    guest: Resources,
    consumed: Resources,
    overhead: Resources
}

#[derive(Debug)]
struct NodeCapacity {
    resources: Resources
}

#[derive(Debug)]
struct Cluster {
    schedulable_control_plane: bool,
    control_plane_node_count: i64,
    worker_node_count: i64,
    worker_node_capacity: NodeCapacity,
    cpuOverCommitRatio: f32,
}

#[derive(Debug)]
struct ClusterResources {
    consumed: Resources,
    overhead: Resources,
    workload: Resources
}

#[derive(Debug)]
struct Reason(String);

#[derive(Debug)]
struct CapacityEstimate {
    resources: ClusterResources,
    reasoning: Vec<Reason>
}

trait ClusterEstimator {
    fn capacity_of(&self, cluster: &Cluster) -> CapacityEstimate;
}

struct HyperConvergedClusterEstimator {}
impl ClusterEstimator for HyperConvergedClusterEstimator {
    fn capacity_of(&self, cluster: &Cluster) -> CapacityEstimate {
        let mut rs = Vec::new();

        let worker_count = if cluster.schedulable_control_plane {
            rs.push(Reason("More capacity due to schedulable control plane nodes".to_string()));
            cluster.worker_node_count + cluster.control_plane_node_count
        } else {
            cluster.worker_node_count
        };

        let total = Resources {
            memory_bytes: cluster.worker_node_capacity.resources.memory_bytes * worker_count,
            cpus: cluster.worker_node_capacity.resources.cpus * worker_count as i64
        };

        rs.push(Reason("HyperConverged clusters have an increased amount of system resource consumption.".to_string()));
        let consumed = Resources {
            // sum by (resource)
            // (kube_pod_container_resource_requests{namespace=~"openshift-.*",
            // resource=~"cpu|memory"}) / 14
            // 14 = count(kube_node_info)
            memory_bytes: worker_count * 20 * GI_B,
            cpus: worker_count as i64 * 8
        };

        rs.push(Reason("The use of ODF benefits from larger buffers.".to_string()));
        let overhead = Resources {
            // avg(sum by (instance) (node_memory_SReclaimable_bytes +
            // node_memory_KReclaimable_bytes))
            memory_bytes: 5 * GI_B,
            cpus: 0
        };
        
        let workload = total - consumed - overhead;

        let cr = ClusterResources {
            consumed: consumed,
            overhead: overhead,
            workload: workload
        };

        CapacityEstimate {resources: cr, reasoning: rs}
    }
}

impl Workloads {
    fn required_resources(&self) -> Resources {
        let c = self.total_count_vm;
        Resources {
            memory_bytes: self.instance_type.guest.memory_bytes * c,
            cpus: self.instance_type.guest.cpus * c as i64
        }
    }
}

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
        cpuOverCommitRatio: 0.1
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
