# ARH - AWS Request Handler

Compiling aws-sdk in rust, especially aws-sdk-ec2 takes ages. Decided to expsoe it via go api that compiles really fast. gRPC is plenty fast and has nice schema definition. Should be good enough :)

## Setup

```
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
```

## Generate

```
protoc --go_out=. --go_opt=paths=source_relative \
    --go-grpc_out=. --go-grpc_opt=paths=source_relative \
    internal/schema/schema.proto
```
