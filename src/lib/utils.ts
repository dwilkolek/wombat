import type { AwsEnv, CommandError } from './types';

export function lambdaAppArn(app: string, env: AwsEnv): string {
	return `wombat::lambdaApp::${app}::${env.toLowerCase()}`;
}

export function cookieSessionProxyArn(address: string, env: AwsEnv): string {
	return `wombat::cookieSessionProxy::${address}::${env.toLowerCase()}`;
}

export function isCommandError(e: unknown): e is CommandError {
	return (e as CommandError).command != null && (e as CommandError).message != null;
}
