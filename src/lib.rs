use std::ops;
use serde::{Serialize, Deserialize};
use display_json::DisplayAsJsonPretty;
use byte_unit::{Byte, AdjustedByte};

/// Represents compute resources (CPU and Memory)
#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, DisplayAsJsonPretty)]
pub struct Resources {
    pub memory: AdjustedByte,
    pub cpus: i64
}

/*
fn<S>(&T, S) -> Result<S::Ok, S::Error> where S: Serializer
self.memory.get_appropriate_unit(true))?;
*
fn serialize_bytes<S>(&_self, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(self.get_appropriate_unit(true))
}*/

fn adjusted_from_bytes(bytes: u128) -> AdjustedByte {
    Byte::from_bytes(bytes).get_appropriate_unit(false)
}

/// Simplify working with Resources
impl<'a, 'b> ops::Add<&'b Resources> for &'a Resources {
    type Output = Resources;
    fn add(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() + _rhs.memory.get_byte().get_bytes()),
            cpus: self.cpus + _rhs.cpus
        }
    }
}
impl<'a, 'b> ops::Sub<&'b Resources> for &'a Resources {
    type Output = Resources;
    fn sub(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() - _rhs.memory.get_byte().get_bytes()),
            cpus: self.cpus - _rhs.cpus
        }
    }
}
impl<'b> ops::Sub<&'b Resources> for Resources {
    type Output = Resources;
    fn sub(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() - _rhs.memory.get_byte().get_bytes()),
            cpus: self.cpus - _rhs.cpus
        }
    }
}
impl<'b> ops::Mul<u64> for Resources {
    type Output = Resources;
    fn mul(self, _rhs: u64) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() * _rhs as u128),
            cpus: self.cpus * _rhs as i64
        }
    }
}

/// Represents a group of workload VMs - as a desired target or available capacity
#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
pub struct Workloads {
    /// How many VMs are compsing this workload
    pub vm_count: u64,
    /// Of what type the VMs are. Currently only a single instanceType per workload is supported
    pub instance_type: InstanceType,
}

impl Workloads {
    pub fn required_capacity(&self) -> Resources {
        self.instance_type.resource_footprint() * self.vm_count
    }
}

/// Represents the instance type (size) of a workload
#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
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
        &(&self.guest + &self.consumed_by_system) + &self.reserved_for_overhead
    }

    /// Calculates how many instances of this type fit into the given cluster resources
    ///
    /// This is a naive calculation, this does not consider to spare resources i.e. for live
    /// migration
    pub fn how_many_fit_into(&self, resources: &ClusterResources) -> (u64, Reason) {
        let avail = &resources.available_to_workloads;
        let req = self.resource_footprint();
        let fit_into_memory = (avail.memory.get_byte().get_bytes() as f64 / req.memory.get_byte().get_bytes() as f64).floor() as u64;
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
#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
pub struct Node {
    /// A human friendly description of the node
    pub description: String,
    /// The capacity of the node (available to workloads, as exposed by kubelet)
    pub capacity: Resources,
    /// Resources consumed by node infrastructure processes (predictable) such as sshd or systemd
    pub consumed_by_system: Resources,
    /// Resources reserved for caches, buffers, and other system depend overheads (difficult to
    /// predict)
    pub reserved_for_overhead: Resources
}

impl Node {
    fn compute_allocatable(&self) -> Resources {
        &self.capacity - &self.consumed_by_system - &self.reserved_for_overhead
    }
}

/// Allow cloning a Node
impl Clone for Node {
    fn clone(&self) -> Self {
        Node{description: self.description.to_owned(),
             capacity: self.capacity.clone(),
             consumed_by_system: self.consumed_by_system.clone(),
             reserved_for_overhead: self.reserved_for_overhead.clone()
             }
    }
}

#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
pub struct ClusterTopology {
    /// If control plane nodes are shared with workloads
    pub schedulable_control_plane: bool,
    /// Optional control plane node spec
    pub control_plane_node: Node,
    /// Type of worker nodes
    pub worker_node: Node,
    /// Ratio of CPU over-commitment, i.e. 1:10 = 1/10 = 0.1
    pub cpu_over_commit_ratio: f32,
}

/// Represents a Cluster
#[derive(Serialize, Deserialize, DisplayAsJsonPretty)]
pub struct Cluster {
    pub topology: ClusterTopology,
    /// Number of control plane nodes
    pub control_plane_node_count: u64,
    /// Number of worker nodes
    pub worker_node_count: u64,
}


impl Cluster {
    /// Compute the cluster resources of this cluster
    pub fn resources(&self) -> ClusterResources {
        let capacity = self.topology.worker_node.capacity * self.worker_node_count;
        let consumed = self.topology.worker_node.consumed_by_system * self.worker_node_count;
        let overhead = self.topology.worker_node.reserved_for_overhead * self.worker_node_count;

        let workload = &capacity - &consumed - &overhead;
        assert_eq!(workload, self.topology.worker_node.compute_allocatable() * self.worker_node_count);

        if self.topology.schedulable_control_plane {
            //rs.push(Reason("More capacity due to schedulable control plane nodes".to_string()));
            let ctl_node_count = self.control_plane_node_count;
            let ctl_node = &self.topology.control_plane_node;

            let ctl_capacity = ctl_node.capacity * ctl_node_count;
            let ctl_consumed = ctl_node.consumed_by_system * ctl_node_count;
            let ctl_overhead = ctl_node.reserved_for_overhead * ctl_node_count;
            let ctl_workload = &ctl_capacity - &ctl_consumed - &ctl_overhead;

            ClusterResources {
                consumed_by_system: &consumed + &ctl_consumed,
                reserved_for_overhead: &overhead + &ctl_overhead,
                available_to_workloads: &workload + &ctl_workload
            }
        } else {
            ClusterResources {
                consumed_by_system: consumed,
                reserved_for_overhead: overhead,
                available_to_workloads: workload
            }
        }
    }
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
    pub reasons: Option<Vec<Reason>>
}

impl Workloads {
    /// Determines how many resources are required to run these workloads
    ///
    /// This is a workload view on resources. This is only determining how many resources are
    /// required by the _workload_, it does not care about potential operational overheads which
    /// the cluster only knows about.
    pub fn required_resources(&self) -> Resources {
        let c = self.vm_count;
        Resources {
            memory: adjusted_from_bytes(self.instance_type.guest.memory.get_byte().get_bytes() * c as u128),
            cpus: self.instance_type.guest.cpus * c as i64
        }
    }

    /// iDetermines if this workload fits into the given cluster resources
    pub fn can_fit_into(&self, resources: &ClusterResources) -> bool {
        let avail = &resources.available_to_workloads;
        let req = self.required_resources();
        let fit_into_memory = avail.memory.get_byte().get_bytes() - req.memory.get_byte().get_bytes() > 0;
        let fit_into_cpu = avail.cpus - req.cpus > 0;
        fit_into_memory && fit_into_cpu
    }
}
