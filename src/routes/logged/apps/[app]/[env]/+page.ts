import type { AwsEnv } from '$lib/types';

/* eslint-disable @typescript-eslint/no-explicit-any */
export const load: any = ({ params }: { params: { app: string; env: AwsEnv } }) => {
	return {
		app: params.app,
		env: params.env
	};
};

export interface AppEnvPage {
	app: string;
	env: AwsEnv;
}
