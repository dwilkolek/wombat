export type SupportLevel = 'Full' | 'Partial' | 'None';

export type SsoProfiles = { [key in AwsEnv]?: SsoProfile };
export type CookieHealth = 'Ok' | 'Stale' | 'Old';
export type CookieHealthMap = { [key in AwsEnv]?: CookieHealth };
export enum BrowserExtensionState {
	Disconnected = 'Disconnected',
	NotSupported = 'NotSupported',
	Outdated = 'Outdated',
	UpToDate = 'UpToDate'
}
export type BrowserExtensionStatus = {
	state: BrowserExtensionState;
	version: string | undefined;
};

export type CookieJarStatus = {
	cookieHealth: CookieHealthMap;
};

export type WombatAwsProfile = {
	name: string;
	profile_base_name: string;
	sso_profiles: SsoProfiles;
	support_level: SupportLevel;
	single_source_profile: boolean;
};
export type SsoProfile = {
	profile_name: string;
	region?: string;
	sso_account_id: string;
	support_level: SupportLevel;
	infra_profiles: InfraProfile[];
	env: AwsEnv;
};

export type InfraProfile = {
	source_profile: string;
	profile_name: string;
	region?: string;
	app: string;
	env: AwsEnv;
};

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
	version: number;
};
type EnvPortMap = { [key: string]: number };
export type WombatProfilePreferences = {
	preferred_environments: AwsEnv[];
	tracked_names: string[];
};
export type UserConfig = {
	id: string | undefined;
	last_used_profile: string | undefined;
	known_profiles: string[];
	dbeaver_path: string | undefined;
	logs_dir: string;
	arn_to_proxy_port_map: EnvPortMap;
	preferences: { [key: string]: WombatProfilePreferences };
};

export type EcsService = {
	env: AwsEnv;
	name: string;
	arn: string;
	cluster_arn: string;
	td_family: string;
};

export type ServiceDetails = {
	arn: string;
	name: string;
	version: string;
	cluster_arn: string;
	env: AwsEnv;
	timestamp: string;
	task_registered_at: string;
	td_family: string;
	td_revision: number;
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
	identifier: string;
	engine: string;
	engine_version: string;
	endpoint: Endpoint;
	subnet_name: string;
	env: AwsEnv;
	arn: string;
	environment_tag: string;
	appname_tag: string;
	cdk_stack_id?: string | null;
	cdk_stack_name?: string | null;
	cdk_logical_id?: string | null;
	master_username?: string | null;
};

export type DatabaseCredentials = {
	arn: string;
	dbname: string;
	password: string;
	username: string;
	auto_rotated: boolean;
	last_changed: string;
};

export type MonitoringConfig = {
	service_arn: string | undefined;
	database_arn: string | undefined;
};

export type CommandError = {
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

	requireSsoProfile: boolean;
};

export type CustomHeader = {
	name: string;
	value: string;
	encodeBase64: boolean;
};

export type TimeUnit = 'minutes' | 'hours' | 'days';
export type Timerange =
	| {
			type: 'relative';
			amount: number;
			unit: TimeUnit;
	  }
	| {
			type: 'absolute';
			from: Date;
			to: Date;
	  };

export enum UpdateButtonState {
	CHECK_DONE,
	DOWNLOADING,
	INSTALLING,
	INSTALLED,
	FAILED
}

export type LogFilter = {
	id: number;
	filter: string;
	services: string[];
	label: string;
};
