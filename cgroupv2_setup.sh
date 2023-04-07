#!/bin/sh

for i in $(seq 1 20)
do
  cgcreate -g memory,cpu:workerd$i
  echo 100M > /sys/fs/cgroup/workerd$i/memory.max
  echo "100000 1000000" > /sys/fs/cgroup/workerd$i/cpu.max
done
