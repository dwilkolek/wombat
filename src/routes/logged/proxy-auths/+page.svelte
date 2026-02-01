<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { open } from '@tauri-apps/plugin-shell';
	import LogFilterForm from '$lib/components/log-filter-form.svelte';
	import ProxyAuthConfigForm from '$lib/components/proxy-auth-config-form.svelte';

	let user = $userStore;
	let dbeaver_path = $state(user?.dbeaver_path ?? '');
	let logs_dir = $state(user?.logs_dir ?? '');
</script>

<svelte:head>
	<title>PROXY AUTH FILTERS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="p-4">
	<ProxyAuthConfigForm />
</div>
