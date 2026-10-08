\set ON_ERROR_STOP on
-- Run only in a disposable local database as its administrator.
BEGIN;
CREATE ROLE authenticated NOLOGIN;
CREATE ROLE anonymous NOLOGIN;
CREATE SCHEMA auth;
CREATE FUNCTION auth.user_id() RETURNS text LANGUAGE sql STABLE AS
    $$ SELECT current_setting('test.user_id', true) $$;
GRANT USAGE ON SCHEMA auth TO authenticated, anonymous;
COMMIT;

\ir ../001_my_wall.sql

BEGIN;
SET LOCAL ROLE authenticated;
SELECT set_config('test.user_id', 'alice', true);
INSERT INTO public.blobatar_avatars (settings) VALUES ('{"name":"Alice"}');
DO $$
BEGIN
    IF (SELECT count(*) FROM public.blobatar_avatars) <> 1 THEN
        RAISE EXCEPTION 'Owner cannot read own avatar';
    END IF;
    BEGIN
        INSERT INTO public.blobatar_avatars (owner_user_id, settings)
        VALUES ('bob', '{}');
        RAISE EXCEPTION 'Owner spoofing was allowed';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        UPDATE public.blobatar_avatars SET owner_user_id = 'bob';
        RAISE EXCEPTION 'Ownership transfer was allowed';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
END;
$$;

SELECT set_config('test.user_id', 'bob', true);
DO $$
DECLARE affected integer;
BEGIN
    IF EXISTS (SELECT 1 FROM public.blobatar_avatars) THEN
        RAISE EXCEPTION 'Another user can read Alice avatar';
    END IF;
    UPDATE public.blobatar_avatars SET settings = '{}';
    GET DIAGNOSTICS affected = ROW_COUNT;
    IF affected <> 0 THEN RAISE EXCEPTION 'Cross-user update allowed'; END IF;
    DELETE FROM public.blobatar_avatars;
    GET DIAGNOSTICS affected = ROW_COUNT;
    IF affected <> 0 THEN RAISE EXCEPTION 'Cross-user delete allowed'; END IF;
END;
$$;

SELECT set_config('test.user_id', 'alice', true);
DO $$
DECLARE affected integer;
BEGIN
    UPDATE public.blobatar_avatars SET settings = '{"name":"Edited"}';
    GET DIAGNOSTICS affected = ROW_COUNT;
    IF affected <> 1 THEN RAISE EXCEPTION 'Owner update failed'; END IF;
    DELETE FROM public.blobatar_avatars;
    GET DIAGNOSTICS affected = ROW_COUNT;
    IF affected <> 1 THEN RAISE EXCEPTION 'Owner delete failed'; END IF;
END;
$$;

SET LOCAL ROLE anonymous;
DO $$
BEGIN
    BEGIN
        PERFORM 1 FROM public.blobatar_avatars;
        RAISE EXCEPTION 'Anonymous read allowed';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
    BEGIN
        INSERT INTO public.blobatar_avatars (settings) VALUES ('{}');
        RAISE EXCEPTION 'Anonymous write allowed';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
END;
$$;
ROLLBACK;
