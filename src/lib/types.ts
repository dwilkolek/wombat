export enum AwsEnv {
	DEVNULL = 'DEVNULL',
	PLAY = 'PLAY',
	LAB = 'LAB',
	DEV = 'DEV',
	DEMO = 'DEMO',
	PROD = 'PROD'
}

export type Cluster = {
	name: string;
	arn: string;
	env: AwsEnv;
};
type EnvPortMap = { [key: string]: number };
export type UserConfig = {
	id: string | undefined;
	last_used_profile: string | undefined;
	known_profiles: string[];
	tracked_names: string[];
	dbeaver_path: string | undefined;
	preffered_environments: AwsEnv[];
	logs_dir: string;
	db_proxy_port_map: { [key: string]: EnvPortMap };
	service_proxy_port_map: { [key: string]: EnvPortMap };
};

export type EcsService = {
	env: AwsEnv;
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
	timestamp: number;
};

export type ServiceDetailsMissing = {
	timestamp: number;
	arn: string;
	name: string;
	error: string;
	env: AwsEnv;
};

export type Endpoint = {
	address: string;
	port: number;
};

export type RdsInstance = {
	name: string;
	normalized_name: string;
	engine: string;
	engine_version: string;
	endpoint: Endpoint;
	env: AwsEnv;
	arn: string;
	environment_tag: string;
	appname_tag: string;
};

export type DatabaseCredentials = {
	dbname: string;
	password: string;
	username: string;
	auto_rotated: boolean;
};

export type MonitoringConfig = {
	service_arn: string | undefined;
	database_arn: string | undefined;
};

export type BError = {
	message: string;
	command: string;
};

export type ProxyAuthConfig = {
	id: number;
	fromApp: string;
	toApp: string;
	env: string;

	authType: string;
	apiPath: string;

	jepsenAuthApi: string | null | undefined;
	jepsenApiName: string | null | undefined;
	jepsenClientId: string | null | undefined;

	basicUser: string | null | undefined;

	secretName: string;
};
