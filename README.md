# Oyster-serverless

Oyster Serverless is a cutting-edge, high-performance serverless computing platform designed to securely execute JavaScript (JS) and WebAssembly (WASM) code in a highly controlled environment. Built using the Rust programming language and Actix Web framework, Oyster serverless leverages the power and security of AWS Nitro Enclaves, Cloudflare workerd runtime, and cgroups to provide unparalleled isolation and protection for the executed code.

## Getting started

<b>Install the following packages : </b>

* build-essential 
* Clang 11+ (e.g. package `clang` on Debian Bullseye)
* libc++ 11+ (e.g. packages `libc++-dev` and `libc++abi-dev` on Debian Bullseye)

`Note : Oyster serverless only works on Ubuntu 22.04 and newer versions due to limitations in the workerd dependency.`

<b>Setup cgroups for workerd :</b>

`Note : serverless supports both version of cgroups (v1 & v2)`

<b>cgroups v1 setup</b>
```
sudo ./cgroupv1_setup.sh
```

Please include the following in the .env file : 

```
CGROUP_VERSION=1
```

<b>cgroups v2 setup</b>
```
sudo ./cgroupv2_setup.sh
```

Please include the following in the .env file : 

```
CGROUP_VERSION=2
```


</br>

## Running serverless application

<b>Run the serverless application :</b>

```
cargo build --release && sudo ./target/x86_64-unknown-linux-musl/release/serverlessrust
```

<b>Make a request to the serveless application :</b>

This transaction hash contains the JavaScript code that finds the prime factors of a given number :
<a href="https://goerli.arbiscan.io/tx/0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113">0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113</a>

Endpoint (POST) : `http://localhost:6000/api/serverless`

JSON body :

```
{
    "tx_hash":"0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113",
    "input": {
        "num":10
    }
}
```
</br>

## Running the tests


<b>Generate the tests : </b>
```
cargo test --no-run
```

<b>Execute the generated test output with root permission :</b>

For example
```
sudo ./target/x86_64-unknown-linux-musl/debug/deps/serverlessrust-4ab6898ba3e00b88
```
