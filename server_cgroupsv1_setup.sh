#!/bin/bash

total_mem_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
total_mem_gb=$(echo "scale=2; $total_mem_kb / 1024 / 1024" | bc)
available_mem_gb=$(echo "$total_mem_gb - 2" | bc )
possible_cgroups=$(echo "$available_mem_gb * 10" | bc | cut -d "." -f 1)
echo "Total system memory: $total_mem_gb GB"
echo "Total available system memory: $available_mem_gb GB"
echo "Possible cgroups : $possible_cgroups"

for i in $(seq 1 $possible_cgroups)
do
  cgcreate -g memory:workerd$i
  echo 100M > /sys/fs/cgroup/memory/workerd$i/memory.limit_in_bytes
done
