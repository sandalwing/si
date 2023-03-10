import _ from "lodash";
import { z } from 'zod';
import Router from "@koa/router";
import { ApiError } from "../lib/api-error";
import {
  completeAuth0TokenExchange, getAuth0LoginUrl, getAuth0LogoutUrl,
} from "../services/auth0.service";
import { SI_COOKIE_NAME, createAuthToken } from '../services/auth.service';
import { setCache, getCache } from "../lib/cache";
import {
  createOrUpdateUserFromAuth0Details,
} from "../services/users.service";
import { validate } from "../lib/validation-helpers";

import { router } from ".";

router.get("/auth/login", async (ctx) => {
  // TODO: can read from querystring info about where request originated
  // so that we can shoot later directly back to the right place, skipping auth portal

  const { randomState, url } = getAuth0LoginUrl();

  // save our auth request in the cache using our random state
  await setCache(
    `auth:start:${randomState}`,
    {
      // here we'll save info about the request
      // like extra query params about where they came from...
    },
    { expiresIn: 300 }, // expire in 5 minutes
  );

  // redirects to Auth0 to actually log in
  ctx.redirect(url);
});

router.get("/auth/login-callback", async (ctx) => {
  // const { code, state } = ctx.request.query;
  // TODO: find a better way to assert/check its not a string array (and make TS happy)
  // (validation tooling should do this)

  const reqQuery = validate(ctx.request.query, z.object({
    // TODO: could check state/code look like valid values
    code: z.string(),
    state: z.string(),
  }));

  // verify `state` matches ours by checking cache (and destroys key so it cannot be used twice)
  const authStartMeta = await getCache(`auth:start:${reqQuery.state}`, true);
  if (!authStartMeta) {
    throw new ApiError('Conflict', 'Oauth state does not match');
  }

  const { profile, token } = await completeAuth0TokenExchange(reqQuery.code);

  const user = await createOrUpdateUserFromAuth0Details(profile);
  // TODO: create/update user, send to posthog, etc...

  // create new JWT used when communicating between the user's browser and _this_ API (via secure http cookie)
  const siToken = createAuthToken(user.id);

  ctx.cookies.set(SI_COOKIE_NAME, siToken, {
    // TODO: verify these settings
    httpOnly: true,
    // secure: true, // IMPORTANT - turn this on if domain is not localhost
    // domain:,
  });

  // ctx.body = { authToken: siToken, profile };

  ctx.redirect(`${process.env.AUTH_PORTAL_URL}/dashboard`);
});

router.get("/auth/logout", async (ctx) => {
  // we wont check if user is logged in because even without an auth cookie from us
  // they could still be logged in on auth0, and forwarding to auth0 logout
  // will log them out there as well

  // clear our auth cookie
  ctx.cookies.set(SI_COOKIE_NAME, null);
  // forward to auth0 which will log them out on auth0
  ctx.redirect(getAuth0LogoutUrl());
});

router.get("/auth/logout-callback", async (ctx) => {
  // console.log("Logged out!");
  // ctx.body = { logout: true };
  ctx.redirect(`${process.env.AUTH_PORTAL_URL}/login`);
});
