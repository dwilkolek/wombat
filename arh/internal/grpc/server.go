package grpc

import (
	"context"
	"fmt"
	"log"

	pb "github.com/dwilkolek/wombat/arh/proto"
)

type server struct {
	pb.UnsafeArhServer
}

func NewServer() *server {
	return &server{}
}

func (s *server) DescribeBastions(_ context.Context, in *pb.DescribeBastionsRequest) (*pb.DescribeBastionsResponse, error) {
	log.Printf("Received: %v", in)
	var instances []*pb.BastionInstance
	for env, name := range pb.Environment_name {
		instances = append(instances, &pb.BastionInstance{
			InstanceId: fmt.Sprintf("Instance Id %s", name),
			Env:        pb.Environment(env),
		})
	}
	return &pb.DescribeBastionsResponse{
		Results: []*pb.BastionInstance{
			&pb.BastionInstance{
				InstanceId: "Instance Id DEV",
				Env:        pb.Environment_DEV,
			},
		},
	}, nil
}
