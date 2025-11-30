import type { AwsEnv, CommandError } from './types';

export function* getFromList<T>(list: T[]): Generator<T> {
	for (let i = 0; i < list.length; i++) {
		yield list[i];
	}
}

export function lambdaAppArn(app: string, env: AwsEnv): string {
	return `wombat::lambdaApp::${app}::${env.toLowerCase()}`;
}

export function cookieSessionProxyArn(address: string, env: AwsEnv): string {
	return `wombat::cookieSessionProxy::${address}::${env.toLowerCase()}`;
}

export function isCommandError(e: unknown): e is CommandError {
	return (e as CommandError).command != null && (e as CommandError).message != null;
}
