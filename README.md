# Oyster-serverless

## Cgroup setup

<b>Create a cgroup for workerd :</b>

```
sudo cgcreate -g memory:workerdcgroup
```

Set memory limit to 100mb for the workerdcgroup (cgroup v1)
```
echo 100M > /sys/fs/cgroup/memory/workerdcgroup/memory.limit_in_bytes
```

Set memory limit to 100mb for the workerdcgroup (cgroup v2)
```
sudo cgset -r memory.max=100M workerdcgroup
```


## Serverless application

<b>Run the serverless application :</b>

```
cargo build --release && sudo ./target/x86_64-unknown-linux-musl/release/serverlessrust
```

<b>Make a request to the serveless application :</b>

Endpoint : `http://localhost:6000/api/serverless`

JSON body :

```
{
    "tx_hash":"0x1fbebe0ca25cc3d98c20e0f9b9f3f17030dc0f632d7791a9d4c57afc3e4524fe"
}
```
