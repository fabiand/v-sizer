use std::ops;
use serde::{Serialize};
use std::fmt;

pub const MI_B: i64 = 1024 * 1024;
pub const GI_B: i64 = MI_B * 1024;
 
#[derive(Clone, Copy, Debug, Serialize)]
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
pub struct Workloads<'a> {
    pub vm_count: i64,
    pub instance_type: &'a InstanceType,
}

#[derive(Debug)]
pub struct InstanceType {
    pub name: String,
    pub guest: Resources,
    pub consumed_by_system: Resources,
    pub reserved_for_overhead: Resources
}

impl InstanceType {
    pub fn resource_footprint(&self) -> Resources {
        Resources {
            memory_bytes: self.guest.memory_bytes + self.consumed_by_system.memory_bytes + self.reserved_for_overhead.memory_bytes,
            cpus: self.guest.cpus + self.consumed_by_system.cpus + self.reserved_for_overhead.cpus
        }
    }
    pub fn how_many_fit_into(&self, estimate: &CapacityEstimate) -> (u64, Reason) {
        let avail = estimate.resources.available_to_workloads;
        let req = self.resource_footprint();
        let fit_into_memory = (avail.memory_bytes as f64 / req.memory_bytes as f64).floor() as u64;
        let fit_into_cpu = (avail.cpus as f64 / req.cpus as f64).floor() as u64;
        if fit_into_memory < fit_into_cpu {
            (fit_into_memory, Reason("Memory constraint".to_string()))
        } else if fit_into_cpu < fit_into_memory {
            (fit_into_cpu, Reason("CPU constraint".to_string()))
        } else {
            (fit_into_cpu, Reason("CPU and memory constratint".to_string()))
        }
    }

}


#[derive(Copy, Clone, Debug, Serialize)]
pub struct Node {
    pub resources: Resources
}

#[derive(Debug, Serialize)]
pub struct Cluster {
    pub schedulable_control_plane: bool,
    pub control_plane_node_count: i64,
    pub worker_node_count: i64,
    pub worker_node: Node,
    pub cpu_over_commit_ratio: f32,
}

impl fmt::Display for Cluster {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string_pretty(&self).unwrap())
    }
}

#[derive(Debug)]
pub struct ClusterResources {
    pub consumed_by_system: Resources,
    pub reserved_for_overhead: Resources,
    pub available_to_workloads: Resources
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
    fn capacity_for(&self, node: &Node, workloads: &Workloads) -> Cluster;
}

pub struct HyperConvergedClusterEstimator {}
impl ClusterEstimator for HyperConvergedClusterEstimator {

    fn capacity_for(&self, node: &Node, workloads: &Workloads) -> Cluster {
        let mut cluster = Cluster {
                schedulable_control_plane: true,
                control_plane_node_count: 3,
                worker_node_count: 0,
                worker_node: *node,
                cpu_over_commit_ratio: 0.1
            };

        loop {
            let fit_into_cluster = workloads.can_fit_into(&self.capacity_of(&cluster));
            // We always add one more node in order to have capacity for LM
            cluster.worker_node_count += 1;
            if fit_into_cluster { break }
        }

        cluster
    }

    fn capacity_of(&self, cluster: &Cluster) -> CapacityEstimate {
        let mut rs = Vec::new();

        let worker_count = if cluster.schedulable_control_plane {
            rs.push(Reason("More capacity due to schedulable control plane nodes".to_string()));
            cluster.worker_node_count + cluster.control_plane_node_count
        } else {
            cluster.worker_node_count
        };

        let total = Resources {
            memory_bytes: cluster.worker_node.resources.memory_bytes * worker_count,
            cpus: cluster.worker_node.resources.cpus * worker_count as i64
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
            consumed_by_system: consumed,
            reserved_for_overhead: overhead,
            available_to_workloads: workload
        };

        CapacityEstimate {resources: cr, reasoning: rs}
    }
}

impl Workloads<'_> {
    pub fn required_resources(&self) -> Resources {
        let c = self.vm_count;
        Resources {
            memory_bytes: self.instance_type.guest.memory_bytes * c,
            cpus: self.instance_type.guest.cpus * c as i64
        }
    }

    pub fn can_fit_into(&self, estimate: &CapacityEstimate) -> bool {
        let avail = estimate.resources.available_to_workloads;
        let req = self.required_resources();
        let fit_into_memory = avail.memory_bytes - req.memory_bytes > 0;
        let fit_into_cpu = avail.cpus - req.cpus > 0;
        fit_into_memory && fit_into_cpu
    }
}
