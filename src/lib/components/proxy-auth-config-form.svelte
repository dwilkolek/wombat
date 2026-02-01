<script lang="ts">
	import { proxyAuthConfigsStore } from '$lib/stores/proxy-auth-configs-store';
	import type { ProxyAuthConfig } from '$lib/types';

	let configs = $state<ProxyAuthConfig[]>([]);
	let initialized = false;

	proxyAuthConfigsStore.subscribe((store) => {
		if (store.length > 0 && !initialized) {
			configs = JSON.parse(JSON.stringify(store)); // Deep copy
			initialized = true;
		}
	});

	let error = $state('');

	function save() {
		try {
			proxyAuthConfigsStore.save(configs);
			error = '';
		} catch (e: any) {
			error = e.message;
		}
	}

	function addConfig() {
		const maxId = configs.reduce((max, c) => Math.max(max, c.id), 0);
		configs.push({
			id: maxId + 1,
			fromApp: '*',
			toApp: '',
			env: 'DEV',
			authType: 'basic',
			apiPath: '/api',
			secretName: '',
			requireSsoProfile: false,
			jepsenAuthApi: null,
			jepsenApiName: null,
			jepsenClientId: null,
			basicUser: null
		});
	}

	function removeConfig(index: number) {
		configs.splice(index, 1);
	}
</script>

<div class="flex flex-col gap-2">
	{#each configs as config, i}
		<div class="collapse collapse-arrow bg-base-100 shadow-sm border border-base-300">
			<input type="checkbox" />
			<div class="collapse-title flex justify-between text-xs items-center p-1 pr-12">
				<div class="font-bold">
					#{config.id}
					{config.fromApp} &rarr; {config.toApp} ({config.env})
				</div>
				<!-- Delete button inside title is tricky with collapse, ignoring for now or using stopPropagation -->
			</div>
			<div class="collapse-content">
				<div class="flex justify-end">
					<button class="btn btn-ghost btn-xs text-error" onclick={() => removeConfig(i)}>
						Delete Config
					</button>
				</div>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2">
					<div class="form-control">
						<label class="label" for={`fromApp-${i}`}
							><span class="label-text">From App</span></label
						>
						<input
							id={`fromApp-${i}`}
							type="text"
							class="input input-sm input-bordered"
							bind:value={config.fromApp}
						/>
					</div>
					<div class="form-control">
						<label class="label" for={`toApp-${i}`}><span class="label-text">To App</span></label>
						<input
							id={`toApp-${i}`}
							type="text"
							class="input input-sm input-bordered"
							bind:value={config.toApp}
						/>
					</div>
					<div class="form-control">
						<label class="label" for={`env-${i}`}><span class="label-text">Env</span></label>
						<select
							id={`env-${i}`}
							class="select select-sm select-bordered"
							bind:value={config.env}
						>
							<option value="DEV">DEV</option>
							<option value="DEMO">DEMO</option>
							<option value="PROD">PROD</option>
							<option value="LAB">LAB</option>
							<option value="PLAY">PLAY</option>
						</select>
					</div>
					<div class="form-control">
						<label class="label" for={`authType-${i}`}
							><span class="label-text">Auth Type</span></label
						>
						<select
							id={`authType-${i}`}
							class="select select-sm select-bordered"
							bind:value={config.authType}
						>
							<option value="basic">Basic</option>
							<option value="jepsen">Jepsen</option>
						</select>
					</div>
					<div class="form-control">
						<label class="label" for={`apiPath-${i}`}
							><span class="label-text">API Path</span></label
						>
						<input
							id={`apiPath-${i}`}
							type="text"
							class="input input-sm input-bordered"
							bind:value={config.apiPath}
						/>
					</div>
					<div class="form-control">
						<label class="label" for={`secretName-${i}`}
							><span class="label-text">Secret Name</span></label
						>
						<input
							id={`secretName-${i}`}
							type="text"
							class="input input-sm input-bordered"
							bind:value={config.secretName}
						/>
					</div>
					<div class="form-control">
						<label class="label cursor-pointer justify-start gap-2" for={`sso-${i}`}>
							<span class="label-text">Require SSO Profile</span>
							<input
								id={`sso-${i}`}
								type="checkbox"
								class="checkbox checkbox-sm"
								bind:checked={config.requireSsoProfile}
							/>
						</label>
					</div>
				</div>

				{#if config.authType === 'jepsen'}
					<div class="divider text-xs">Jepsen Settings</div>
					<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
						<div class="form-control">
							<label class="label" for={`jepsenAuthApi-${i}`}
								><span class="label-text">Auth API</span></label
							>
							<input
								id={`jepsenAuthApi-${i}`}
								type="text"
								class="input input-sm input-bordered"
								bind:value={config.jepsenAuthApi}
							/>
						</div>
						<div class="form-control">
							<label class="label" for={`jepsenApiName-${i}`}
								><span class="label-text">API Name</span></label
							>
							<input
								id={`jepsenApiName-${i}`}
								type="text"
								class="input input-sm input-bordered"
								bind:value={config.jepsenApiName}
							/>
						</div>
						<div class="form-control">
							<label class="label" for={`jepsenClientId-${i}`}
								><span class="label-text">Client ID</span></label
							>
							<input
								id={`jepsenClientId-${i}`}
								type="text"
								class="input input-sm input-bordered"
								bind:value={config.jepsenClientId}
							/>
						</div>
					</div>
				{/if}

				{#if config.authType === 'basic'}
					<div class="divider text-xs">Basic Auth Settings</div>
					<div class="form-control">
						<label class="label" for={`basicUser-${i}`}
							><span class="label-text">Basic User</span></label
						>
						<input
							id={`basicUser-${i}`}
							type="text"
							class="input input-sm input-bordered"
							bind:value={config.basicUser}
						/>
					</div>
				{/if}
			</div>
		</div>
	{/each}

	<button class="btn btn-outline btn-sm btn-block" onclick={addConfig}> + Add Proxy Config </button>

	{#if error}
		<div class="text-error mt-2">{error}</div>
	{/if}

	<button class="btn btn-primary mt-4" onclick={save}> Save Proxy Configs </button>
</div>
