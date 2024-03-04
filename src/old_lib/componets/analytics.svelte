<script lang="ts">
	import { page } from '$app/stores';
	import { writable } from 'svelte/store';
	import { version } from '$app/environment';
	import { userStore } from '$lib/stores/user-store';
	let last_push = writable<any>();
	$: user = userStore;
	$: {
		if (
			typeof gtag !== 'undefined' &&
			JSON.stringify($last_push) !=
				JSON.stringify({
					page_title: document.title,
					page_path: $page.url.pathname,
					user_id: $user.id,
					appName: 'wombat',
					appVersion: version
				})
		) {
			last_push.set({
				page_title: document.title,
				page_path: $page.url.pathname,
				user_id: $user.id,
				appName: 'wombat',
				appVersion: version
			});
			gtag('config', 'G-VD6DFXWQH0', {
				page_title: `${document.title} v${version}`,
				page_path: $page.url.pathname,
				user_id: $user.id,
				appName: 'wombat',
				appVersion: version
			});
		}
	}
</script>

<svelte:head>
	<script async src="https://www.googletagmanager.com/gtag/js?id=G-VD6DFXWQH0">
	</script>
	<script>
		window.dataLayer = window.dataLayer || [];

		function gtag() {
			dataLayer.push(arguments);
		}

		gtag('js', new Date());
		gtag('config', 'G-VD6DFXWQH0');
	</script>
</svelte:head>
