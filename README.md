# Oyster-serverless

Oyster Serverless is a cutting-edge, high-performance serverless computing platform designed to securely execute JavaScript (JS) and WebAssembly (WASM) code in a highly controlled environment. Built using the Rust and Actix Web framework, Oyster serverless leverages the power and security of AWS Nitro Enclaves, Cloudflare workerd runtime, and cgroups to provide unparalleled isolation and protection for the executed code.

## Getting started

<b>Install the following packages : </b>

* build-essential 
* libc++1
* cgroup-tools

`Note : Oyster serverless only works on Ubuntu 22.04 and newer versions due to limitations in the workerd dependency.`

<b>cgroups v2 setup</b>
```
sudo ./cgroupv2_setup.sh
```

## Running serverless application

<b>Run the serverless application :</b>

```
cargo build --release --target x86_64-unknown-linux-musl && sudo ./target/x86_64-unknown-linux-musl/release/serverlessrust
```

<b>Make a request to the serveless application :</b>

This transaction hash contains the JavaScript code that finds the prime factors of a given number :
<a href="https://goerli.arbiscan.io/tx/0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113">0xc7d9122f583971d4801747ab24cf3e83984274b8d565349ed53a73e0a547d113</a>

Endpoint (POST) : `http://Y7MREL2YHFY5JAAXI6VSJTZ6QOMEE5FY2VSTJHWVHJZ6BJKH2EJQ.localhost:6000`

JSON body :

```
{
    "num":10
}
```

## Running the tests

The tests need root privileges internally. They should work as long as the shell has sudo cached, a simple `sudo echo` will ensure that.

```
sudo echo && cargo test -- --test-threads 1
```
