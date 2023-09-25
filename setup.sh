#!/bin/sh

# setting an address for loopback
ifconfig lo 127.0.0.1
ifconfig

# adding a default route
ip route add default via 127.0.0.1 dev lo
route -n

# iptables rules to route traffic to transparent proxy
iptables -A OUTPUT -t nat -p tcp --dport 1:65535 ! -d 127.0.0.1  -j DNAT --to-destination 127.0.0.1:1200
iptables -t nat -A POSTROUTING -o lo -s 0.0.0.0 -j SNAT --to-source 127.0.0.1
iptables -L -t nat

# generate identity key
/app/keygen --secret /app/id.sec --public /app/id.pub

# your custom setup goes here

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
  echo "50000" > /sys/fs/cgroup/cpu/workerd$i/cpu.cfs_quota_us
done

# starting supervisord
cat /etc/supervisord.conf
/app/supervisord