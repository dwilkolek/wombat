export enum Env {
	DEVNULL = 'DEVNULL',
	PLAY = 'PLAY',
	LAB = 'LAB',
	DEV = 'DEV',
	DEMO = 'DEMO',
	PROD = 'PROD'
}
export type Cluster = {
	arn: String;
	env: Env;
};
export type UserConfig = {
	last_used_profile: string | undefined;
	known_profiles: string[];
	favourite_service_names: string[];
	favourite_db_arns: string[];
	dbeaver_path: string | undefined;
};

export type EcsService = {
	name: string;
	arn: string;
	cluster_arn: string;
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
