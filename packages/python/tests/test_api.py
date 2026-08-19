import unittest

from baeld_agent import (
    BaeldAgent,
    BaeldPhaseModel,
    Session,
    connect_browser_use,
    with_model_wait,
)


class PublicApiTest(unittest.TestCase):
    def test_api_is_importable(self):
        self.assertTrue(callable(BaeldAgent.connect))
        self.assertTrue(callable(BaeldPhaseModel))
        self.assertTrue(callable(connect_browser_use))
        self.assertTrue(callable(with_model_wait))
        self.assertEqual(Session("s", "c", "safe", "ephemeral").id, "s")


class PhaseModelTest(unittest.IsolatedAsyncioTestCase):
    async def test_wraps_exact_model_call(self):
        class Agent:
            previous_phase = "observing"
            events = []

            async def waiting_for_model(self, expected_wait_ms, **kwargs):
                self.events.append(("waiting", expected_wait_ms, kwargs))

            async def acting(self):
                self.events.append(("acting",))

        class Model:
            model = "test-model"
            provider = "test"
            name = "test"
            model_name = "test-model"

            async def ainvoke(self, messages, output_format=None, **kwargs):
                agent.events.append(("invoke", messages))
                return "result"

        agent = Agent()
        wrapped = BaeldPhaseModel(Model(), agent, expected_wait_ms=1234)
        self.assertEqual(await wrapped.ainvoke(["prompt"]), "result")
        self.assertEqual([event[0] for event in agent.events], ["waiting", "invoke", "acting"])
