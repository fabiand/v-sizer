use std::ops;

pub const MI_B: i64 = 1024 * 1024;
pub const GI_B: i64 = MI_B * 1024;
 
#[derive(Clone, Copy, Debug)]
pub struct Resources {
    pub memory_bytes: i64,
    pub cpus: i64
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
pub struct Workloads {
    pub total_count_vm: i64,
    pub instance_type: InstanceType,
}

#[derive(Debug)]
pub struct InstanceType {
    pub name: String,
    pub guest: Resources,
    pub consumed: Resources,
    pub overhead: Resources
}

#[derive(Debug)]
pub struct NodeCapacity {
    pub resources: Resources
}

#[derive(Debug)]
pub struct Cluster {
    pub schedulable_control_plane: bool,
    pub control_plane_node_count: i64,
    pub worker_node_count: i64,
    pub worker_node_capacity: NodeCapacity,
    pub cpu_over_commit_ratio: f32,
}

#[derive(Debug)]
pub struct ClusterResources {
    pub consumed: Resources,
    pub overhead: Resources,
    pub workload: Resources
}

#[derive(Debug)]
pub struct Reason(String);

#[derive(Debug)]
pub struct CapacityEstimate {
    pub resources: ClusterResources,
    pub reasoning: Vec<Reason>
}

pub trait ClusterEstimator {
    fn capacity_of(&self, cluster: &Cluster) -> CapacityEstimate;
}

pub struct HyperConvergedClusterEstimator {}
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
    pub fn required_resources(&self) -> Resources {
        let c = self.total_count_vm;
        Resources {
            memory_bytes: self.instance_type.guest.memory_bytes * c,
            cpus: self.instance_type.guest.cpus * c as i64
        }
    }
}
