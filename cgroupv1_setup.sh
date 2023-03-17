#!/bin/sh
cgcreate -g memory:workerd1
echo 100M > /sys/fs/cgroup/memory/workerd1/memory.limit_in_bytes

cgcreate -g memory:workerd2
echo 100M > /sys/fs/cgroup/memory/workerd2/memory.limit_in_bytes

cgcreate -g memory:workerd3
echo 100M > /sys/fs/cgroup/memory/workerd3/memory.limit_in_bytes

cgcreate -g memory:workerd4
echo 100M > /sys/fs/cgroup/memory/workerd4/memory.limit_in_bytes

cgcreate -g memory:workerd5
echo 100M > /sys/fs/cgroup/memory/workerd5/memory.limit_in_bytes

echo "\nCGROUP_VERSION=1" >> .env