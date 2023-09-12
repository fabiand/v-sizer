use std::ops;
use serde::{Serialize, Deserialize};
use display_json::DisplayAsJsonPretty;
use byte_unit::{Byte, AdjustedByte};

/// Represents compute resources (CPU and Memory)
#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug, DisplayAsJsonPretty)]
pub struct Resources {
    pub memory: AdjustedByte,
    pub cpus: i64,
    pub vcpus: Option<i64>
}

fn adjusted_from_bytes(bytes: u128) -> AdjustedByte {
    Byte::from_bytes(bytes).get_appropriate_unit(false)
}

/// Simplify working with Resources
impl<'a, 'b> ops::Add<&'b Resources> for &'a Resources {
    type Output = Resources;
    fn add(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() + _rhs.memory.get_byte().get_bytes()),
            cpus: self.cpus + _rhs.cpus,
            vcpus: match (self.vcpus, _rhs.vcpus) { (Some(v), Some(rv)) => Some(v + rv), (Some(v), None) => Some(v), (None, Some(rv)) => Some(rv), (None, None) => None}
        }
    }
}
impl<'a, 'b> ops::Sub<&'b Resources> for &'a Resources {
    type Output = Resources;
    fn sub(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() - _rhs.memory.get_byte().get_bytes()),
            cpus: self.cpus - _rhs.cpus,
            vcpus: match (self.vcpus, _rhs.vcpus) { (Some(v), Some(rv)) => Some(v - rv), (Some(v), None) => Some(v), (None, Some(rv)) => Some(rv), (None, None) => None}
        }
    }
}
impl<'b> ops::Sub<&'b Resources> for Resources {
    type Output = Resources;
    fn sub(self, _rhs: &'b Resources) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() - _rhs.memory.get_byte().get_bytes()),
            cpus: self.cpus - _rhs.cpus,
            vcpus: match (self.vcpus, _rhs.vcpus) { (Some(v), Some(rv)) => Some(v - rv), (Some(v), None) => Some(v), (None, Some(rv)) => Some(rv), (None, None) => None}
        }
    }
}
impl<'b> ops::Mul<u64> for Resources {
    type Output = Resources;
    fn mul(self, _rhs: u64) -> Resources {
        Resources {
            memory: adjusted_from_bytes(self.memory.get_byte().get_bytes() * _rhs as u128),
            cpus: self.cpus * _rhs as i64,
            vcpus: match self.vcpus { Some(v) => Some(v * _rhs as i64), _ => None }
        }
    }
}

impl<'b> ops::Mul<u64> for &Resources {
    type Output = Resources;
    fn mul(self, _rhs: u64) -> Resources {
        *self * _rhs
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
            (fit_into_memory, "Memory constraint".to_string())
        } else if fit_into_cpu < fit_into_memory {
            (fit_into_cpu, "CPU constraint".to_string())
        } else {
            (fit_into_cpu, "CPU and memory constratint".to_string())
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

#[derive(Clone, Serialize, Deserialize, DisplayAsJsonPretty)]
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
    pub fn for_topology_and_workload(topology: ClusterTopology, workloads: Workloads) -> ReasonedResult<Cluster> {
        let mut reasons = Vec::new();

        let mut cluster = Cluster {
            topology: topology.clone(),
            control_plane_node_count: 3,
            worker_node_count: 0
        };

        loop {
            let fit_into_cluster = workloads.can_fit_into(&cluster.resources());
            // We always add one more node in order to have capacity for LM
            cluster.worker_node_count += 1;
            if fit_into_cluster.result == true { break }
        }
        reasons.push("One additional node is getting included on top of the required ones, in order to enable full-node drains required for updating the cluster".to_string());

        ReasonedResult {
            result: cluster,
            reasons: reasons
        }
    }
    /// Compute the cluster resources of this cluster
    pub fn resources(&self) -> ClusterResources {
        let worker_node = &self.topology.worker_node;

        let mut consumed = worker_node.consumed_by_system * self.worker_node_count;
        let mut overhead = worker_node.reserved_for_overhead * self.worker_node_count;
        let mut workload = worker_node.compute_allocatable() * self.worker_node_count;

        if self.topology.schedulable_control_plane {
            //rs.push(Reason("More capacity due to schedulable control plane nodes".to_string()));
            let ctl_node_count = self.control_plane_node_count;
            let ctl_node = &self.topology.control_plane_node;

            let ctl_capacity = ctl_node.capacity * ctl_node_count;
            let ctl_consumed = ctl_node.consumed_by_system * ctl_node_count;
            let ctl_overhead = ctl_node.reserved_for_overhead * ctl_node_count;
            let ctl_workload = &ctl_capacity - &ctl_consumed - &ctl_overhead;

            consumed = &consumed + &ctl_consumed;
            overhead = &overhead + &ctl_overhead;
            workload = &workload + &ctl_workload;
        }

        workload.vcpus = Some((workload.cpus as f32 * (1.0 / self.topology.cpu_over_commit_ratio)) as i64);

        ClusterResources {
            consumed_by_system: consumed,
            reserved_for_overhead: overhead,
            available_to_workloads: workload
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
//#[derive(Serialize, DisplayAsJsonPretty)]
//pub struct Reason(String);
type Reason = String;

/// Represents an estimated cluster capacity
#[derive(Serialize, DisplayAsJsonPretty)]
pub struct ReasonedResult<T> {
    pub result: T,
    pub reasons: Vec<Reason>
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
            cpus: self.instance_type.guest.cpus * c as i64,
            vcpus: match self.instance_type.guest.vcpus { Some(v) => Some(v * c as i64), None => None }
        }
    }

    /// Determines if this workload fits into the given cluster resources
    pub fn can_fit_into(&self, resources: &ClusterResources) -> ReasonedResult<bool> {
        let avail = &resources.available_to_workloads;
        let req = self.required_resources();
        let avail_mem = avail.memory.get_byte().get_bytes();
        let req_mem = req.memory.get_byte().get_bytes();

        if avail_mem < req_mem { return ReasonedResult{result: false, reasons: vec!["Constrained by memory".to_string()]} };
        if avail.cpus < req.cpus { return ReasonedResult{result: false, reasons: vec!["Constrained by pCPU".to_string()]} };
        //if avail.vcpus < req.vcpus { return ReasoneResult(result: false, reasons: vec!["Constrained by vCPU"]) };
        ReasonedResult{result: true, reasons: vec![]}
    }
}
