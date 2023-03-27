<script lang="ts">
	import { goto } from '$app/navigation';
	import { state } from './store';
	let storeProfile = state.profile;
	let storeErr = state.error;
	let inputProfile = 'developer';
	state.profile.subscribe((profile) => {
		inputProfile = profile ?? 'developer';
	});
	let loading = false;
</script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="hero max-h-screen min-h-screen bg-base-200">
	<div class="hero-content flex-col">
		<div class="text-center">
			<h1 class="text-5xl font-bold">Hello!</h1>
			<p class="py-6">Wombat is friendly app that aims to make your life less miserable 😎</p>
		</div>
		<div class="card flex-shrink-0 w-full max-w-sm shadow-2xl bg-base-100">
			<div class="card-body">
				<div class="form-control">
					<label class="label" for="aws-profile">
						<span class="label-text">AWS profile</span>
					</label>
					<input
						id="aws-profile"
						type="text"
						placeholder="AWS profile"
						class="input input-bordered w-full max-w-xs"
						bind:value={inputProfile}
						required
					/>
				</div>

				<div class="form-control mt-6">
					<button
						class="btn btn-accent"
						disabled={loading}
						on:click={async () => {
							loading = true;
							console.log(inputProfile, loading, storeProfile);
							await state.start(inputProfile);
							loading = false;
							goto(`/logged/ecs`, { replaceState: true });
						}}
					>
						{loading ? 'Preloading...' : 'Get Start'}</button
					>
				</div>
				<div>
					<p>{$storeErr ?? ''}</p>
				</div>
			</div>
		</div>
	</div>
</div>
