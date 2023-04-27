## WriteHasher

Hash the data being written to a writer.  
Supports `tokio::io::AsyncWrite`, `futures::io::AsyncWrite`, `std::io::Write`.  

#### Example
```rust
extern crate sha2;
use sha2::Sha256;
use write_hasher::WriteHasher;

let mut file = std::fs::File::open("Cargo.toml").unwrap();
let dest = std::io::sink();
let dest = WriteHasher::<Sha256, _>::new(dest);
std::io::copy(&mut file, &mut dest).unwrap();
let hash = dest.finalize();
```

You can use async functions as well as std functions for this as well.
