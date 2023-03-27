<script lang="ts">
  import { Environment, state } from "./store";
  const environments = Object.keys(Environment).map(
    (env) => env as Environment
  );
  const env = state.env;
  let storeProfile = state.profile;
  let storeErr = state.error;
  let inputProfile = "developer";
  state.profile.subscribe((profile) => {
    inputProfile = profile ?? "developer";
  });
  let loading = false;
</script>

<div class="navbar bg-base-100 ">
  <input
    type="text"
    placeholder="AWS profile"
    class="input input-bordered w-full max-w-xs text-accent-content"
    bind:value={inputProfile}
  />
  <div class="dropdown dropdown-end">
    <label tabindex="0" class="btn m-1 text-accent-content">{$env}</label>
    <ul
      tabindex="0"
      class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52"
    >
      {#each environments as env}
        <li><a on:click={() => state.selectEnvironment(env)}>{env}</a></li>
      {/each}
    </ul>
  </div>

  <button
    class="btn btn-accent"
    disabled={loading || inputProfile == $storeProfile}
    on:click={async () => {
      loading = true;
      console.log(inputProfile, loading, storeProfile);
      await state.start(inputProfile);
      loading = false;
    }}
  >
    Start</button
  >
  {$storeErr ?? ""}
</div>
