package grpc

import (
	"context"
	"log"
	"os/exec"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/ec2"
	"github.com/aws/aws-sdk-go-v2/service/ec2/types"
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

	cfg, err := config.LoadDefaultConfig(context.TODO(), config.WithSharedConfigProfile(in.Profile), config.WithRegion("eu-west-1"))
	if err != nil {
		log.Fatalf("unable to load SDK config, %v", err)
		cmd := exec.Command("aws", "sso", "login", "--profile", in.Profile)
		_, err := cmd.Output()
		if err != nil {
			log.Fatalf("unable to load SDK config, %v", err)
		}
		return s.DescribeBastions(context.TODO(), in)
	}

	client := ec2.NewFromConfig(cfg)
	var instances []*pb.BastionInstance
	tagFilterName := "tag:Name"
	result, err := client.DescribeInstances(context.TODO(), &ec2.DescribeInstancesInput{
		Filters: []types.Filter{
			{
				Name: &tagFilterName,
				Values: []string{
					"*-bastion*",
				},
			},
		},
	})
	if err != nil {
		log.Fatalf("unable to load SDK config, %v", err)
	}
	for _, reservation := range result.Reservations {
		for _, instance := range reservation.Instances {
			var env pb.Environment
			for _, tag := range instance.Tags {
				if *tag.Key == "Environment" {
					switch *tag.Value {
					case "dev":
						env = pb.Environment_DEV
					case "demo":
						env = pb.Environment_DEMO
					case "prod":
						env = pb.Environment_PROD
					}
				}
			}
			instances = append(instances, &pb.BastionInstance{
				InstanceId: *instance.InstanceId,
				Env:        env,
			})
		}
	}
	return &pb.DescribeBastionsResponse{
		Results: instances,
	}, nil
}
