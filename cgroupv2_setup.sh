#!/bin/sh

set -e

for i in $(seq 1 20)
do
  cgcreate -g memory,cpu:workerd_$i
  echo 100M > /sys/fs/cgroup/workerd_$i/memory.max
  echo "50000 1000000" > /sys/fs/cgroup/workerd_$i/cpu.max
done
