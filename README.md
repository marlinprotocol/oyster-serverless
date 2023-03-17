# Oyster-serverless

### Getting started

<b>Install the following packages : </b>

* build-essential 
* Clang 11+ (e.g. package `clang` on Debian Bullseye)
* libc++ 11+ (e.g. packages `libc++-dev` and `libc++abi-dev` on Debian Bullseye)

<b>Setup cgroups for workerd :</b>

```
sudo ./cgroup_setup.sh
```


### Running serverless application

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
