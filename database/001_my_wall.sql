BEGIN;

CREATE TABLE public.blobatar_avatars (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id text NOT NULL DEFAULT auth.user_id(),
    settings jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT settings_object CHECK (jsonb_typeof(settings) = 'object'),
    CONSTRAINT settings_size CHECK (octet_length(settings::text) <= 1048576)
);

CREATE INDEX blobatar_avatars_owner_updated
    ON public.blobatar_avatars (owner_user_id, updated_at DESC, id);

ALTER TABLE public.blobatar_avatars ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.blobatar_avatars FORCE ROW LEVEL SECURITY;

CREATE POLICY own_avatars ON public.blobatar_avatars
    FOR ALL TO authenticated
    USING (owner_user_id = (SELECT auth.user_id()))
    WITH CHECK (owner_user_id = (SELECT auth.user_id()));

REVOKE ALL ON public.blobatar_avatars FROM PUBLIC, anonymous, authenticated;
GRANT USAGE ON SCHEMA public TO authenticated;
GRANT SELECT, DELETE ON public.blobatar_avatars TO authenticated;
GRANT INSERT (settings), UPDATE (settings) ON public.blobatar_avatars TO authenticated;

CREATE FUNCTION public.blobatar_touch_updated_at()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    NEW.updated_at := clock_timestamp();
    RETURN NEW;
END;
$$;

REVOKE ALL ON FUNCTION public.blobatar_touch_updated_at() FROM PUBLIC;
CREATE TRIGGER blobatar_avatars_updated_at
    BEFORE UPDATE ON public.blobatar_avatars
    FOR EACH ROW EXECUTE FUNCTION public.blobatar_touch_updated_at();

COMMIT;
