## Useful docs

https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/rds-cloudwatch-events.sample.html

```
RDS-EVENT-0005 DB instance created.
RDS-EVENT-0170 DB cluster created.
```

https://stackoverflow.com/questions/71317287/error-build-rust-for-linux-in-macos-openssl

https://github.com/orelvis15/cvm/blob/master/build_debug.sh

https://github.com/sfackler/rust-openssl/issues/1331

cargo build --release --target x86_64-unknown-linux-musl

## MacOs SSL Compile

```
For compilers to find openssl@3 you may need to set:
  export LDFLAGS="-L/usr/local/opt/openssl@3/lib"
  export CPPFLAGS="-I/usr/local/opt/openssl@3/include"

For pkg-config to find openssl@3 you may need to set:
  export PKG_CONFIG_PATH="/usr/local/opt/openssl@3/lib/pkgconfig"


ln -s /usr/local/opt/openssl\@3.1/lib/pkgconfig/libcrypto.pc libcrypto.pc
ln -s /usr/local/opt/openssl\@3.1/lib/pkgconfig/libssl.pc libssl.pc
ln -s /usr/local/opt/openssl\@3.1/lib/pkgconfig/openssl.pc openssl.pc

/usr/local/lib/pkgconfig


export OPENSSL_LIB_DIR=/usr/local/opt/openssl\@3.1/lib/
export OPENSSL_DIR=/usr/local/opt/openssl\@3.1/
export OPENSSL_INCLUDE_DIR=/usr/local/opt/openssl\@3.1/include/
```
