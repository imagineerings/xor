ALTER TABLE public.channel_files
    ADD COLUMN download_count bigint NOT NULL DEFAULT 0;
