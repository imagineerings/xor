ALTER TABLE public.channel_messages
    ADD COLUMN priority smallint NOT NULL DEFAULT 0;

CREATE INDEX index_channel_messages_on_priority
    ON public.channel_messages USING btree (priority);
