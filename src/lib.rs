use std::ops;
use serde::{Serialize};
use display_json::DisplayAsJsonPretty;
use byte_unit::Byte;

pub const MI_B: u64 = 1024 * 1024;
pub const GI_B: u64 = MI_B * 1024;
 
 /// Represents compute resources (CPU and Memory)
#[derive(Clone, Serialize, DisplayAsJsonPretty)]
pub struct Resources {
    pub memory: Byte,
    pub cpus: i64
}

/// Simplify working with Resources
impl<'a, 'b> ops::Add<&'b Resources> for &'a Resources {
    type Output = Resources;
    fn add(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: Byte::from_bytes(self.memory.get_bytes() + _rhs.memory.get_bytes()),
            cpus: self.cpus + _rhs.cpus
        }
    }
}

/// Simplify working with Resources
impl<'a, 'b> ops::Sub<&'b Resources> for &'a Resources {
    type Output = Resources;
    fn sub(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: Byte::from_bytes(self.memory.get_bytes() - _rhs.memory.get_bytes()),
            cpus: self.cpus - _rhs.cpus
        }
    }
}

/// Simplify working with Resources
impl<'b> ops::Sub<&'b Resources> for Resources {
    type Output = Resources;
    fn sub(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: Byte::from_bytes(self.memory.get_bytes() - _rhs.memory.get_bytes()),
            cpus: self.cpus - _rhs.cpus
        }
    }
}

/// Represents a group of workload VMs - as a desired target or available capacity
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct Workloads<'a> {
    /// How many VMs are compsing this workload
    pub vm_count: i64,
    /// Of what type the VMs are. Currently only a single instanceType per workload is supported
    pub instance_type: &'a InstanceType,
}

/// Represents the instance type (size) of a workload
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct InstanceType {
    /// The name of the instanceType
    pub name: String,
    /// Resources available to the guest/workload (predictable)
    pub guest: Resources,
    /// Resources consumed by per-VM infrastructure processes (predictable)
    pub consumed_by_system: Resources,
    /// Resources reserved for caches, buffers, and workload depend overheads (difficult to
    /// predict)
    pub reserved_for_overhead: Resources
}

impl InstanceType {
    /// Returns the amount of resources which will be consumed on the host by running one instance
    /// of this instanceType
    ///
    /// There are three resource consumers:
    /// 1. Resources provided to the guest. These are predictable
    /// 2. Resources required to run VM related processes (such as qemu, and libvirt)
    /// 3. Resources required to cope with caches such as page cache and slab
    ///
    /// In general the resource footprint of a VM is larger than the resources available to the
    /// guest. This is becaue a VM is supported by host sided user- and kernel-space processes such
    /// as qemu, or slab. In addition to this, there are workload and configuration dependent
    /// overheads, such as buffers to hold data until they get written to the destination.
    pub fn resource_footprint(&self) -> Resources {
        /*Resources {
            memory: Byte::from_bytes(self.guest.memory.get_bytes() + self.consumed_by_system.memory.get_bytes() + self.reserved_for_overhead.memory.get_bytes()),
            cpus: self.guest.cpus + self.consumed_by_system.cpus + self.reserved_for_overhead.cpus
        }*/
        &(&self.guest + &self.consumed_by_system) + &self.reserved_for_overhead
    }

    /// Calculates how many instances of this type fit into the given cluster resources
    ///
    /// This is a naive calculation, this does not consider to spare resources i.e. for live
    /// migration
    pub fn how_many_fit_into(&self, resources: &ClusterResources) -> (u64, Reason) {
        let avail = &resources.available_to_workloads;
        let req = self.resource_footprint();
        let fit_into_memory = (avail.memory.get_bytes() as f64 / req.memory.get_bytes() as f64).floor() as u64;
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

/// Represents a physical node
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct Node {
    /// A human friendly description of the node
    pub description: String,
    /// The capacity of the node (available to workloads, as exposed by kubelet)
    pub resources: Resources
}

/// Allow cloning a Node
impl Clone for Node {
    fn clone(&self) -> Self {
        Node{description: self.description.to_owned(),
             resources: self.resources.clone()}
    }
}

/// Represents a Cluster
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct Cluster {
    /// A human friendly description of the cluster
    pub description: String,
    /// If control plane nodes are shared with workloads
    pub schedulable_control_plane: bool,
    /// Number of control plane nodes
    pub control_plane_node_count: u16,
    /// Number of worker nodes
    pub worker_node_count: u16,
    /// Type of worker nodes
    pub worker_node: Node,
    /// Ratio of CPU over-commitment, i.e. 1:10 = 1/10 = 0.1
    pub cpu_over_commit_ratio: f32,
}

/// Represents a detailed view on the resource distribution in a Cluster
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct ClusterResources {
    /// Resources consumed by system processes (such as qemu, systemd, libvirt, KubeVirt)
    pub consumed_by_system: Resources,
    /// Resources reserved for workload overheads, such as PageCache, BufferCache, SLAB, …
    pub reserved_for_overhead: Resources,
    /// Resources available to the workloads, thus the resources as seen by the guest of a VM
    pub available_to_workloads: Resources
}

/// Represents a Reason
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct Reason(String);

/// Represents an estimated cluster capacity
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct ClusterCapacityEstimate {
    pub resources: ClusterResources,
    pub reasoning: Vec<Reason>
}

pub trait ClusterEstimator {
    /// Estimate the resource capacity of a given cluster
    fn capacity_of(&self, cluster: &Cluster) -> ClusterCapacityEstimate;
    /// Estimate what cluster is needed given a node type and workloads
    fn capacity_for(&self, node: &Node, workloads: &Workloads) -> Cluster;
}

/// Represents a Hyper Converged (HC) cluster
pub struct HyperConvergedClusterEstimator {}

/// Implement the ClusterEstimator trait for HC Clusters
impl ClusterEstimator for HyperConvergedClusterEstimator {
    fn capacity_for(&self, node: &Node, workloads: &Workloads) -> Cluster {
        let mut cluster = Cluster {
                description: "Cluster with sufficient capacity".to_owned(),
                schedulable_control_plane: true,
                control_plane_node_count: 3,
                worker_node_count: 0,
                worker_node: node.clone(),
                cpu_over_commit_ratio: 0.1
            };

        loop {
            let fit_into_cluster = workloads.can_fit_into(&self.capacity_of(&cluster).resources);
            // We always add one more node in order to have capacity for LM
            cluster.worker_node_count += 1;
            if fit_into_cluster { break }
        }

        cluster
    }

    fn capacity_of(&self, cluster: &Cluster) -> ClusterCapacityEstimate {
        let mut rs = Vec::new();

        let worker_count = if cluster.schedulable_control_plane {
            rs.push(Reason("More capacity due to schedulable control plane nodes".to_string()));
            cluster.worker_node_count + cluster.control_plane_node_count
        } else {
            cluster.worker_node_count
        };

        let total = Resources {
            memory: Byte::from_bytes(cluster.worker_node.resources.memory.get_bytes() * worker_count),
            cpus: cluster.worker_node.resources.cpus * worker_count as i64
        };

        rs.push(Reason("HyperConverged clusters have an increased amount of system resource consumption.".to_string()));
        let consumed = Resources {
            // sum by (resource)
            // (kube_pod_container_resource_requests{namespace=~"openshift-.*",
            // resource=~"cpu|memory"}) / 14
            // 14 = count(kube_node_info)
            memory: worker_count * Byte::from_str("20 GiB").unwrap(),
            cpus: worker_count as i64 * 8
        };

        rs.push(Reason("The use of ODF benefits from larger buffers.".to_string()));
        let overhead = Resources {
            // avg(sum by (instance) (node_memory_SReclaimable_bytes +
            // node_memory_KReclaimable_bytes))
            memory: Byte::from_str("5 GiB").unwrap(),
            cpus: 0
        };
        
        let workload = &total - &consumed - &overhead;

        let cr = ClusterResources {
            consumed_by_system: consumed,
            reserved_for_overhead: overhead,
            available_to_workloads: workload
        };

        ClusterCapacityEstimate {resources: cr, reasoning: rs}
    }
}

impl Workloads<'_> {
    /// Determines how many resources are required to run these workloads
    ///
    /// This is a workload view on resources. This is only determining how many resources are
    /// required by the _workload_, it does not care about potential operational overheads which
    /// the cluster only knows about.
    pub fn required_resources(&self) -> Resources {
        let c = self.vm_count;
        Resources {
            memory: Byte::from_bytes(self.instance_type.guest.memory.get_bytes() as u32 * c as u16),
            cpus: self.instance_type.guest.cpus * c
        }
    }

    /// iDetermines if this workload fits into the given cluster resources
    pub fn can_fit_into(&self, resources: &ClusterResources) -> bool {
        let avail = &resources.available_to_workloads;
        let req = self.required_resources();
        let fit_into_memory = avail.memory.get_bytes() - req.memory.get_bytes() > 0;
        let fit_into_cpu = avail.cpus - req.cpus > 0;
        fit_into_memory && fit_into_cpu
    }
}
