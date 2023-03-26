<script lang="ts">
  import { Environment, state } from "./store";
  const environments = Object.keys(Environment).map(
    (env) => env as Environment
  );
  const env = state.env;
</script>

<div class="navbar bg-base-100">
  <div class="flex-1">
    <h1 class="normal-case text-xl" />
  </div>
  <div class="flex-none">
    <ul class="menu menu-horizontal px-1">
      <li><a>ECS</a></li>
      <li><a>RDS</a></li>
    </ul>
  </div>

  <div class="navbar-end">
    <div class="dropdown dropdown-end">
      <label tabindex="0" class="btn m-1">{$env}</label>
      <ul
        tabindex="0"
        class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52"
      >
        {#each environments as env}
          <li><a on:click={() => state.selectEnvironment(env)}>{env}</a></li>
        {/each}
      </ul>
    </div>
  </div>
</div>
