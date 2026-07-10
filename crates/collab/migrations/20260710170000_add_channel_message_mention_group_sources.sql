ALTER TABLE public.channel_message_mentions
    ADD COLUMN source_group_id bigint REFERENCES public.user_groups(id) ON DELETE SET NULL;

CREATE INDEX index_channel_message_mentions_on_source_group_id
    ON public.channel_message_mentions USING btree (source_group_id);
