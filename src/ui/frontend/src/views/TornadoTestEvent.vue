<template>
  <Container>
    <h1 class="title">Test Event</h1>
    <div class="columns">

      <div class="column">

        <div class="card">
          <header class="card-header">
            <p class="card-header-title">
              Test Data
            </p>
          </header>
          <div class="card-content">
            <div class="content">
              <div class="field">
                <label class="label">Event type</label>
                <div class="control">
                  <input v-model="event.event.type" class="input" type="text">
                </div>
              </div>

              <div class="field">
                <label class="label">Created timestamp (linux epoch)</label>
                <div class="control">
                  <input v-model.number="event.event.created_ms" class="input" type="number">
                </div>
              </div>

              <div class="field">
                <label class="label">Payload (JSON)</label>
                <div class="control">
                  <textarea :value="payload" @input="updatePayload($event.target.value)" class="textarea"></textarea>
                </div>
              </div>

              <div class="field is-grouped">
                <div class="control">
                  <button @click="sendEvent()" class="button is-link">Submit</button>
                </div>
              </div>
            </div>
          </div>
        </div>

      </div>

      <div class="column is-two-thirds">
        <div class="card">
          <header class="card-header">
            <p class="card-header-title">
              Test Result
            </p>
          </header>
          <div class="card-content">
            <div class="content">
              <pre>{{ result }}</pre>
            </div>
          </div>
        </div>
      </div>

    </div>
  </Container>
</template>

<script lang="ts">
import { Component, Vue } from 'vue-property-decorator';
import { SendEventRequestDto, ProcessedEventDto, ProcessType } from '@/generated/dto';
import { postSendEvent } from '@/api/api';

@Component({})
export default class TornadoTestEvent extends Vue {

  public event: SendEventRequestDto = {
    event: {
      type: 'my type',
      created_ms: 0,
      payload: {
        value_one: 'something',
        value_two: 'something_else',
      },
    },
    process_type: ProcessType.SkipActions,
  };

  public result: ProcessedEventDto|null = null;


  get payload(): string {
    return JSON.stringify(this.event.event.payload);
  }

  public updatePayload(payload: string) {
    this.event.event.payload = JSON.parse(payload);
  }

  public sendEvent() {
    // console.log(`send event: ${JSON.stringify(this.event)}`);
    postSendEvent(this.event)
    .then((result) => {
      this.result = result;
    });
  }

}
</script>
