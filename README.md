# Oyster-serverless

Oyster Serverless is a cutting-edge, high-performance serverless computing platform designed to securely execute JavaScript (JS) and WebAssembly (WASM) code in a highly controlled environment. Built using the Rust and Actix Web framework, Oyster serverless leverages the power and security of AWS Nitro Enclaves, Cloudflare workerd runtime, and cgroups to provide unparalleled isolation and protection for the executed code.

## Getting started

`Note : The Oyster serverless application only works inside the enclave. The current setup relies on a temporary storage server designed for testing.`

<b>Install the following packages : </b>

* build-essential 
* musl-tools
* libc++1

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

<b>Generate a release build :</b>

```
cargo build --release
```

<b>Run the binary file within Oyster by utilizing supervisord and proxy the server using a vsock-to-ip proxy : </b>
```
#Server
[program:server]
command= /app/serverlessrust
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stdout
stderr_logfile_maxbytes=0

#Proxy for server
[program:my-server-proxy]
command=/app/vsock-to-ip --vsock-addr 88:6000 --ip-addr 127.0.0.1:6000
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stdout
stderr_logfile_maxbytes=0
```

`Note : oyster-serverless requires attestation server to be running inside oyster `

<b>Make a request to the serveless application :</b>

Endpoint (POST) : `http://localhost:6000/api/serverless`

JSON body :

```
{
    "code_id":"test",
    "input": {
        "num":100
    }
}
```

</br>

## Testing serverless application

<b>Generate the tests : </b>

```
cargo test --no-run
```

`Note : oyster-serverless tests requires attestation server to be running inside oyster `

<b>Run the test binary file within Oyster by utilizing supervisord : </b>
```
#Server tests
[program:serverlesstest]
command= /app/serverlessrust-c5133baa1a8a70aa
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stdout
stderr_logfile_maxbytes=0
autorestart=false
```
