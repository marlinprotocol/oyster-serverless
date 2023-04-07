#!/bin/bash

#Fetching total memory in enclave and generating cgroups
total_mem_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
total_mem_mb=$(echo "scale=2; $total_mem_kb / 1024" | bc)
available_mem_mb=$(echo "$total_mem_mb - 2000" | bc )
#Setting the memory limit for each cgroup and calculating the number of cgroups possible
cgroup_memory_size=100
possible_cgroups=$(echo "$available_mem_mb / $cgroup_memory_size" | bc | cut -d "." -f 1)

echo "Total system memory: $total_mem_mb mb"
echo "Total available system memory: $available_mem_mb mb"
echo "Possible cgroups : $possible_cgroups"

for i in $(seq 1 $possible_cgroups)
do
  cgcreate -g memory,cpu:workerd$i
  echo "$cgroup_memory_size"M > /sys/fs/cgroup/memory/workerd$i/memory.limit_in_bytes
  echo "1000000" > /sys/fs/cgroup/cpu/workerd$i/cpu.cfs_period_us
  echo "100000" > /sys/fs/cgroup/cpu/workerd$i/cpu.cfs_quota_us
done
