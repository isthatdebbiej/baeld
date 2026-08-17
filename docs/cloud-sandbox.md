# Disposable cloud sandbox

Use cloud compute only after the local WSL gate produces a signal worth pursuing.
The development sandbox is not the source of headline measurements.

## Cost hierarchy

1. WSL2, concurrency one: free and suitable for development only.
2. A new-customer Google Cloud trial: suitable for the pilot when trial quota allows it.
3. A Spot VM: suitable for checkpointed smoke and pilot blocks; preempted blocks are infrastructure-invalid.
4. An on-demand VM: required for the final fixed-host measurement windows.

The permanent Google Cloud `e2-micro` free tier is not suitable. It is too small and
uses shared compute. A budget alert is only a notification; it is not a hard spending cap.

## GCP development sandbox

Create one Compute Engine VM in the console:

- Ubuntu 24.04 LTS, x86-64.
- `n2-standard-4` (4 vCPU, 16 GiB) for smoke and a reduced pilot.
- 40–50 GB balanced persistent disk.
- Standard provisioning initially; Spot is acceptable after cleanup is proven.
- No GPU, HTTP load balancer, Kubernetes cluster, or browser-cloud service.
- Do not enable public HTTP/HTTPS firewall rules. SSH access is sufficient.

The repository is private. In the VM's SSH terminal:

```bash
sudo apt-get update
sudo apt-get install -y gh git
gh auth login --hostname github.com --git-protocol https --web
gh repo clone isthatdebbiej/baeld -- --branch dev
cd baeld
bash scripts/setup-ubuntu.sh
```

Validate before benchmarking:

```bash
cd ~/baeld
bash scripts/run-cloud-gate.sh
```

Baeld must be the direct command in each delegated transient scope. Do not enter an
interactive delegated shell and do not use `cargo run` for an experiment: the shell
or Cargo process would remain in the cgroup root and prevent controller delegation.

For a reduced cloud pilot, copy `experiments/pilot.toml`, keep concurrency at one,
and retain at least five paired blocks. Do not silently call it the final experiment.

## Final host

Use one on-demand 8-vCPU/32-GiB x86-64 VM for both final time windows. Record the
exact provider, region, zone, machine family, CPU model, image, disk type, and VM
lifetime in `experiments/LOG.md`. Baeld records the in-guest environment automatically.

Before deleting the VM:

```bash
tar -czf baeld-results.tgz results report
```

Download the archive, verify it locally, then delete both the VM and its persistent
disk. Stopping a VM does not necessarily stop disk or reserved-address charges.
