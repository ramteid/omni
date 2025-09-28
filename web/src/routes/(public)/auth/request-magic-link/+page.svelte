<script>
    import { enhance } from '$app/forms'
    import { page } from '$app/stores'

    let loading = false
    let email = ''
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50">
    <div class="w-full max-w-md space-y-8">
        <div class="text-center">
            <h2 class="mt-6 text-3xl font-bold text-gray-900">Request Magic Link</h2>
            <p class="mt-2 text-sm text-gray-600">
                Enter your work email to receive a secure login link
            </p>
        </div>

        <form
            method="POST"
            class="mt-8 space-y-6"
            use:enhance={() => {
                loading = true
                return async ({ update }) => {
                    await update()
                    loading = false
                }
            }}>
            <div class="space-y-4">
                <div>
                    <label for="email" class="block text-sm font-medium text-gray-700">
                        Email address
                    </label>
                    <input
                        id="email"
                        name="email"
                        type="email"
                        autocomplete="email"
                        required
                        bind:value={email}
                        class="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 placeholder-gray-400 shadow-sm focus:border-blue-500 focus:ring-blue-500 focus:outline-none"
                        placeholder="you@company.com" />
                </div>
            </div>

            {#if $page.form?.error}
                <div class="rounded-md bg-red-50 p-4">
                    <div class="text-sm text-red-700">
                        {$page.form.error}
                    </div>
                </div>
            {/if}

            <div>
                <button
                    type="submit"
                    disabled={loading}
                    class="group relative flex w-full justify-center rounded-md border border-transparent bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50">
                    {#if loading}
                        <span class="absolute inset-y-0 left-0 flex items-center pl-3">
                            <div
                                class="h-5 w-5 animate-spin rounded-full border-2 border-white border-t-transparent">
                            </div>
                        </span>
                        Sending...
                    {:else}
                        Send Magic Link
                    {/if}
                </button>
            </div>

            <div class="text-center">
                <a href="/login" class="text-sm text-blue-600 hover:text-blue-500">
                    Back to login
                </a>
            </div>
        </form>
    </div>
</div>
