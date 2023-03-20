# Oyster-serverless

## Getting started

<b>Install the following packages : </b>

* build-essential 
* Clang 11+ (e.g. package `clang` on Debian Bullseye)
* libc++ 11+ (e.g. packages `libc++-dev` and `libc++abi-dev` on Debian Bullseye)

<b>Setup cgroups for workerd :</b>

`Note : serverless supports both version of cgroups (v1 & v2)`

<b>cgroups v1 </b>
```
sudo ./cgroupv1_setup.sh
```

Please include the following in the .env file : 

```
CGROUP_VERSION=1
```

<b>cgroups v2 </b>
```
sudo ./cgroupv2_setup.sh
```

Please include the following in the .env file : 

```
CGROUP_VERSION=2
```



## Running serverless application

<b>Run the serverless application :</b>

```
cargo build --release && sudo ./target/x86_64-unknown-linux-musl/release/serverlessrust
```

<b>Make a request to the serveless application :</b>

Endpoint (POST) : `http://localhost:6000/api/serverless`

JSON body :

```
{
    "tx_hash":"0xe53e82630ca8ea3386ec6acf556850255f64d39eb1ef53194e90dd288b0f89ba",
    "input": {
        "name":"marlin"
    }
}
```
