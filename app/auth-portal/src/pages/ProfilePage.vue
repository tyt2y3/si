<!-- eslint-disable vue/no-v-html -->
<template>
  <div>
    <template v-if="loadUserReqStatus.isPending">
      <Icon name="loader" size="xl" />
    </template>
    <template v-else-if="loadUserReqStatus.isError">
      <ErrorMessage :requestStatus="loadUserReqStatus" />
    </template>
    <template v-else-if="draftUser">
      <div class="flex gap-xl">
        <div class="w-[35%] flex items-center pl-md">
          <Stack>
            <!-- this text only shows the first time / user is in onboarding -->
            <template v-if="isOnboarding">
              <RichText>
                <h2>Welcome To System Initiative!</h2>
                <p>Please enter your profile information.</p>
              </RichText>
            </template>
            <!-- this is the default text -->
            <template v-else>
              <RichText>
                <h2>Update Your Profile</h2>
                <p>Use this page to update your profile info.</p>
              </RichText>
            </template>
          </Stack>
        </div>

        <form class="grow my-md px-md">
          <Stack>
            <ErrorMessage :requestStatus="updateUserReqStatus" />
            <VormInput
              v-if="draftUser.pictureUrl || storeUser?.pictureUrl"
              label="Profile Image"
              type="container"
            >
              <div
                v-if="draftUser.pictureUrl"
                class="flex flex-row items-center gap-sm"
              >
                <img
                  :src="draftUser.pictureUrl"
                  class="rounded-full w-xl h-xl"
                />
                <VButton
                  tone="destructive"
                  size="xs"
                  variant="ghost"
                  @click="clearPicture"
                  >Clear Picture
                </VButton>
              </div>
              <div
                v-else-if="storeUser?.pictureUrl"
                class="h-xl items-center flex flex-row gap-sm"
              >
                <div class="italic text-sm">No image set.</div>
                <VButton
                  tone="action"
                  size="xs"
                  variant="ghost"
                  @click="restorePicture"
                  >Restore Picture
                </VButton>
              </div>
            </VormInput>
            <Tiles columns="2" spacing="sm" columnsMobile="1">
              <VormInput
                v-model="draftUser.firstName"
                label="First Name"
                autocomplete="given-name"
                placeholder="Your first name"
                :regex="ALLOWED_INPUT_REGEX"
              />
              <VormInput
                v-model="draftUser.lastName"
                label="Last Name"
                autocomplete="last-name"
                placeholder="Your last name"
                :regex="ALLOWED_INPUT_REGEX"
              />
            </Tiles>
            <VormInput
              v-model="draftUser.nickname"
              label="Nickname"
              autocomplete="username"
              required
              placeholder="This name will be shown in the application"
              :regex="ALLOWED_INPUT_REGEX"
            />
            <VormInput
              v-model="draftUser.email"
              label="Email"
              type="email"
              autocomplete="email"
              required
              disabled
              placeholder="ex: yourname@somewhere.com"
            />
            <VormInput
              v-model="draftUser.discordUsername"
              label="Discord Username"
              name="discord_username"
              placeholder="ex: eggscellent OR eggscellent#1234"
              :regex="DISCORD_TAG_REGEX"
              regexMessage="Invalid discord tag"
              class="pb-xs"
            >
              <template #instructions>
                <div class="text-neutral-700 dark:text-neutral-200 italic">
                  Entering your username will help us to give you technical
                  support
                  <a href="" class="underline text-action-500">on our Discord</a
                  >.
                </div>
              </template>
            </VormInput>
            <VormInput
              v-model="draftUser.githubUsername"
              label="Github Username"
              name="github_username"
              placeholder="ex: devopsdude42"
              :regex="GITHUB_USERNAME_REGEX"
              regexMessage="Invalid github username"
            />

            <VButton
              iconRight="chevron--right"
              :disabled="validationState.isError"
              :requestStatus="updateUserReqStatus"
              loadingText="Saving your profile..."
              successText="Updated your profile!"
              tone="action"
              variant="solid"
              @click="saveHandler"
            >
              Save
            </VButton>
          </Stack>
        </form>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
/* eslint-disable @typescript-eslint/no-non-null-assertion */

import * as _ from "lodash-es";
import { useRouter } from "vue-router";
import { computed, onBeforeMount, ref, watch } from "vue";
import {
  ErrorMessage,
  Icon,
  Tiles,
  Stack,
  useValidatedInputGroup,
  VButton,
  VormInput,
  RichText,
} from "@si/vue-lib/design-system";
import { useHead } from "@vueuse/head";
import { useAuthStore, User } from "@/store/auth.store";
import { tracker } from "@/lib/posthog";
import { API_HTTP_URL } from "@/store/api";
import { useWorkspacesStore } from "@/store/workspaces.store";
import { ALLOWED_INPUT_REGEX } from "@/lib/validations";

const GITHUB_USERNAME_REGEX = /^[a-z\d](?:[a-z\d]|-(?=[a-z\d])){0,38}$/i;
const DISCORD_TAG_REGEX =
  /^(?!(discord|here|everyone))(((?!.*\.\.)(([\w.]{2,32})))|[^@#:]{2,32}#[\d]{4})$/i;

const { validationState, validationMethods } = useValidatedInputGroup();
const authStore = useAuthStore();
const workspacesStore = useWorkspacesStore();
const router = useRouter();

const loadUserReqStatus = authStore.getRequestStatus("LOAD_USER");
const checkAuthReqStatus = authStore.getRequestStatus("CHECK_AUTH");
const updateUserReqStatus = authStore.getRequestStatus("UPDATE_USER");

const storeUser = computed(() => authStore.user);
const draftUser = ref<User>();
const isOnboarding = ref<boolean>();

useHead({ title: "Profile" });

function resetDraftUser() {
  draftUser.value = _.cloneDeep(storeUser.value!);
}

watch(storeUser, resetDraftUser, { immediate: true });

function checkUserOnboarding() {
  if (storeUser.value && isOnboarding.value === undefined) {
    isOnboarding.value = !storeUser.value.onboardingDetails?.reviewedProfile;
  }
}

watch(storeUser, checkUserOnboarding, { immediate: true });

onBeforeMount(() => {
  // normally when landing on this page, we should probably make sure we have the latest profile info
  // but we already load user info with CHECK_AUTH so can skip if it was just loaded
  // (this will likely go away if we start fetching more profile info than what gets fetched while checking auth)
  if (+checkAuthReqStatus.value.lastSuccessAt! > +new Date() - 10000) {
    return;
  }

  // eslint-disable-next-line @typescript-eslint/no-floating-promises
  authStore.LOAD_USER();
});

const saveHandler = async () => {
  if (validationMethods.hasError()) return;

  // if this is first time, we will take them off profile page after save
  const updateReq = await authStore.UPDATE_USER(draftUser.value!);
  if (updateReq.result.success && isOnboarding.value) {
    if (
      storeUser.value &&
      storeUser.value.emailVerified &&
      !storeUser.value.auth0Id.startsWith("auth0")
    ) {
      // We only want to send this event when a user has signed up and
      // we captured a verified email for them. We will only ever have a
      // user with an auth0Id of auth0 if it's not using SSO. So we will
      // force that user through a manual verification and capture the user
      // at that stage
      // This means we won't ever be sending badly formed data to our CRM
      // or billing
      // This is also the place we would trigger the creation of a Billing user
      tracker.trackEvent("initial_profile_set", {
        email: draftUser.value?.email,
        githubUsername: draftUser.value?.githubUsername,
        discordUsername: draftUser.value?.discordUsername,
        firstName: draftUser.value?.firstName,
        lastName: draftUser.value?.lastName,
      });

      await authStore.BILLING_INTEGRATION();
    }

    const completeProfileReq = await authStore.COMPLETE_PROFILE({});
    if (completeProfileReq.result.success) {
      if (authStore.user?.emailVerified && workspacesStore.defaultWorkspace) {
        tracker.trackEvent("workspace_launcher_widget_click");
        window.location.href = `${API_HTTP_URL}/workspaces/${workspacesStore.defaultWorkspace.id}/go`;
      } else {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        router.push({ name: "workspaces" });
      }
    }
  } else {
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    router.push({ name: "workspaces" });
  }
};
const clearPicture = () => {
  if (draftUser.value) {
    draftUser.value.pictureUrl = null;
  }
};
const restorePicture = () => {
  if (draftUser.value && storeUser.value) {
    draftUser.value.pictureUrl = storeUser.value.pictureUrl;
  }
};
</script>
