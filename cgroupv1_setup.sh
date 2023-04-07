#!/bin/sh

for i in $(seq 1 20)
do
  cgcreate -g memory,cpu:workerd$i
  echo 100M > /sys/fs/cgroup/memory/workerd$i/memory.limit_in_bytes
  echo "1000000" > /sys/fs/cgroup/cpu/workerd$i/cpu.cfs_period_us
  echo "100000" > /sys/fs/cgroup/cpu/workerd$i/cpu.cfs_quota_us
done