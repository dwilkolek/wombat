package main

import (
	"fmt"
	"log"
	"net"

	arh "github.com/dwilkolek/wombat/arh/internal/grpc"
	"github.com/dwilkolek/wombat/arh/proto"
	"google.golang.org/grpc"
)

func main() {
	lis, err := net.Listen("tcp", fmt.Sprintf(":%d", 6666))
	if err != nil {
		log.Fatalf("failed to listen: %v", err)
	}
	s := grpc.NewServer()
	proto.RegisterArhServer(s, arh.NewServer())
	log.Printf("server listening at %v", lis.Addr())
	if err := s.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}
