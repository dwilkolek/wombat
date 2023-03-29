export type UserConfig = {
	last_used_profile: string | undefined;
	known_profiles: string[];
	monitored: MonitoringConfig[];
	dbeaver_path: string | undefined;
};

export type EcsService = {
	name: string;
	service_arn: string;
	cluster_arn: string;
};

export type Endpoint = {
	address: string;
	port: number;
};

export type DbInstance = {
	db_name: string;
	endpoint: Endpoint;
	db_instance_arn: string;
	environment_tag: string;
	appname_tag: string;
};

export type MonitoringConfig = {
	service_arn: string | undefined;
	database_arn: string | undefined;
};
