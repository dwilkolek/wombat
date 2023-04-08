export enum AwsEnv {
	DEVNULL = 'DEVNULL',
	PLAY = 'PLAY',
	LAB = 'LAB',
	DEV = 'DEV',
	DEMO = 'DEMO',
	PROD = 'PROD'
}

export type Cluster = {
	arn: string;
	env: AwsEnv;
};
export type UserConfig = {
	id: string | undefined;
	last_used_profile: string | undefined;
	known_profiles: string[];
	ecs: string[];
	rds: string[];
	dbeaver_path: string | undefined;
};

export type EcsService = {
	name: string;
	arn: string;
	cluster_arn: string;
};

export type ServiceDetails = {
	arn: string;
	name: string;
	version: string;
	cluster_arn: string;
	env: AwsEnv;
};

export type Endpoint = {
	address: string;
	port: number;
};

export type DbInstance = {
	name: string;
	endpoint: Endpoint;
	arn: string;
	environment_tag: string;
	appname_tag: string;
};

export type MonitoringConfig = {
	service_arn: string | undefined;
	database_arn: string | undefined;
};

export type BError = {
	message: string;
	command: string;
};
