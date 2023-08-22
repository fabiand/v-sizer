This is a rough framwork to build sizers for different OCP topologies.
It can estimate a cluster size for a target workload _or_ provide an estimate of how many workloads fit onto a given cluster topology.

# Run

```console
$ cargo run
   Compiling serde_derive v1.0.173
   Compiling proc-macro2 v1.0.66
   Compiling serde v1.0.173
   Compiling serde_json v1.0.104
   Compiling unicode-ident v1.0.11
   Compiling itoa v1.0.9
   Compiling ryu v1.0.15
   Compiling syn v1.0.109
   Compiling utf8-width v0.1.6
   Compiling quote v1.0.31
   Compiling byte-unit v4.0.19
   Compiling display_json v0.2.1
   Compiling sizer v0.1.0 (/var/home/fabiand/work/openshift/sizer)
    Finished dev [unoptimized + debuginfo] target(s) in 11.41s
     Running `target/debug/sizer`
Cluster: {
  "description": "",
  "schedulable_control_plane": false,
  "control_plane_node_count": 3,
  "worker_node_count": 3,
  "worker_node": {
    "description": "Worker node",
    "resources": {
      "memory": 274877906944,
      "cpus": 128
    }
  },
  "cpu_over_commit_ratio": 0.1
}
Estimated cluster capacity: {
  "resources": {
    "consumed_by_system": {
      "memory": 64424509440,
      "cpus": 24
    },
    "reserved_for_overhead": {
      "memory": 5368709120,
      "cpus": 0
    },
    "available_to_workloads": {
      "memory": 754840502272,
      "cpus": 360
    }
  },
  "reasoning": [
    "HyperConverged clusters have an increased amount of system resource consumption.",
    "The use of ODF benefits from larger buffers."
  ]
}
Workloads: {
  "vm_count": 100,
  "instance_type": {
    "name": "u1.medium",
    "guest": {
      "memory": 4294967296,
      "cpus": 8
    },
    "consumed_by_system": {
      "memory": 209715200,
      "cpus": 1
    },
    "reserved_for_overhead": {
      "memory": 0,
      "cpus": 0
    }
  }
}
Estimated workload capacity: {
  "memory": 754840502272,
  "cpus": 360
}
Avail - req: {
  "memory": 325343772672,
  "cpus": -440
}
Workload fit into estimate? false
Workload how many fit into estimate? 40 constrained by "CPU constraint"
Cluster estimate for workload: {
  "description": "Cluster with sufficient capacity",
  "schedulable_control_plane": true,
  "control_plane_node_count": 3,
  "worker_node_count": 5,
  "worker_node": {
    "description": "Worker node",
    "resources": {
      "memory": 274877906944,
      "cpus": 128
    }
  },
  "cpu_over_commit_ratio": 0.1
}
```

# Plan

1. Provide different opologies
2. Move example to tests/example
