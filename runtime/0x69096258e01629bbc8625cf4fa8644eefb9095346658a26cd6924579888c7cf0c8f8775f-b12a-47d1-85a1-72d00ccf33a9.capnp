using Workerd = import "/workerd/workerd.capnp";

    const helloWorldExample :Workerd.Config = (
      services = [ (name = "main", worker = .helloWorld) ],
      sockets = [ ( name = "http", address = "*:38197", http = (), service = "main" ) ]
    );
    
    const helloWorld :Workerd.Worker = (
      serviceWorkerScript = embed "0x69096258e01629bbc8625cf4fa8644eefb9095346658a26cd6924579888c7cf0c8f8775f-b12a-47d1-85a1-72d00ccf33a9.js",
      compatibilityDate = "2022-09-16",
    );