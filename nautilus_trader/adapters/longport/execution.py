# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

"""
Longport execution client implementation.
"""

import asyncio
from decimal import Decimal

from longport.openapi import (
    Config,
    TradeContext,
    OrderSide as LPOrderSide,
    OrderType as LPOrderType,
    OrderStatus as LPOrderStatus,
    TimeInForceType as LPTimeInForce,
)

from nautilus_trader.adapters.longport.config import LongportExecClientConfig
from nautilus_trader.adapters.longport.common.constants import LONGPORT_VENUE
from nautilus_trader.adapters.longport.providers import LongportInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.common.enums import LogColor
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.core.uuid import UUID4
from nautilus_trader.execution.reports import ExecutionMassStatus
from nautilus_trader.execution.reports import FillReport
from nautilus_trader.execution.reports import OrderStatusReport
from nautilus_trader.execution.reports import PositionStatusReport
from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.live.execution_client import GenerateOrderStatusReports
from nautilus_trader.live.execution_client import GenerateFillReports
from nautilus_trader.live.execution_client import GeneratePositionStatusReports
from nautilus_trader.model.enums import LiquiditySide
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.enums import OrderStatus
from nautilus_trader.model.enums import OrderType
from nautilus_trader.model.enums import OmsType
from nautilus_trader.model.enums import AccountType
from nautilus_trader.model.enums import PositionSide
from nautilus_trader.model.enums import TimeInForce
from nautilus_trader.model.identifiers import AccountId
from nautilus_trader.model.identifiers import ClientId
from nautilus_trader.model.identifiers import ClientOrderId
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.identifiers import TradeId
from nautilus_trader.model.identifiers import VenueOrderId
from nautilus_trader.model.objects import Money
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity


# ---------------------------------------------------------------------------
# Enum mapping helpers
# ---------------------------------------------------------------------------

def _map_lp_order_status(lp_status) -> OrderStatus:
    """Map Longport OrderStatus to Nautilus OrderStatus."""
    name = type(lp_status).__name__
    _map = {
        "NotReported": OrderStatus.SUBMITTED,
        "ReplacedNotReported": OrderStatus.SUBMITTED,
        "ProtectedNotReported": OrderStatus.SUBMITTED,
        "VarietiesNotReported": OrderStatus.SUBMITTED,
        "WaitToNew": OrderStatus.SUBMITTED,
        "New": OrderStatus.ACCEPTED,
        "WaitToReplace": OrderStatus.PENDING_UPDATE,
        "PendingReplace": OrderStatus.PENDING_UPDATE,
        "Replaced": OrderStatus.ACCEPTED,
        "PartialFilled": OrderStatus.PARTIALLY_FILLED,
        "WaitToCancel": OrderStatus.PENDING_CANCEL,
        "PendingCancel": OrderStatus.PENDING_CANCEL,
        "Canceled": OrderStatus.CANCELED,
        "Rejected": OrderStatus.REJECTED,
        "Expired": OrderStatus.EXPIRED,
        "Filled": OrderStatus.FILLED,
        "Unknown": OrderStatus.SUBMITTED,
    }
    return _map.get(name, OrderStatus.SUBMITTED)


def _map_lp_order_side(lp_side) -> OrderSide:
    name = type(lp_side).__name__
    return OrderSide.BUY if name == "Buy" else OrderSide.SELL


def _map_lp_order_type(lp_type) -> OrderType:
    name = type(lp_type).__name__
    _map = {
        "LO": OrderType.LIMIT,
        "ELO": OrderType.LIMIT,
        "ALO": OrderType.LIMIT,
        "SLO": OrderType.LIMIT,
        "MO": OrderType.MARKET,
        "AO": OrderType.MARKET,
        "ODD": OrderType.MARKET,
        "LIT": OrderType.LIMIT_IF_TOUCHED,
        "MIT": OrderType.MARKET_IF_TOUCHED,
        "TSLPAMT": OrderType.TRAILING_STOP_LIMIT,
        "TSLPPCT": OrderType.TRAILING_STOP_LIMIT,
        "TSMAMT": OrderType.TRAILING_STOP_MARKET,
        "TSMPCT": OrderType.TRAILING_STOP_MARKET,
    }
    return _map.get(name, OrderType.LIMIT)


def _map_lp_time_in_force(lp_tif) -> TimeInForce:
    name = type(lp_tif).__name__
    _map = {
        "Day": TimeInForce.DAY,
        "GTC": TimeInForce.GTC,
        "GTD": TimeInForce.GTD,
        "IOC": TimeInForce.IOC,
        "FOK": TimeInForce.FOK,
    }
    return _map.get(name, TimeInForce.DAY)


def _lp_symbol_to_instrument_id(symbol: str) -> InstrumentId:
    """Convert Longport symbol (e.g. 'AAPL.US') to Nautilus InstrumentId."""
    return InstrumentId.from_str(f"{symbol}.LONGPORT")


def _datetime_to_nanos(dt) -> int:
    """Convert datetime to nanoseconds since epoch."""
    if dt is None:
        return 0
    import calendar
    return int(calendar.timegm(dt.timetuple()) * 1e9)


# ---------------------------------------------------------------------------
# Execution client
# ---------------------------------------------------------------------------

class LongportExecutionClient(LiveExecutionClient):
    """
    Longport execution client.

    Parameters
    ----------
    loop : asyncio.AbstractEventLoop
    msgbus : MessageBus
    cache : Cache
    clock : LiveClock
    instrument_provider : LongportInstrumentProvider
    config : LongportExecClientConfig
    name : str, optional
    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: LongportInstrumentProvider,
        config: LongportExecClientConfig,
        name: str | None = None,
    ) -> None:
        super().__init__(
            loop=loop,
            client_id=ClientId(name or LONGPORT_VENUE.value),
            venue=LONGPORT_VENUE,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
            oms_type=OmsType.NETTING,
            base_currency=None,
            account_type=AccountType.MARGIN,
        )
        self._config = config
        self._trade_ctx: TradeContext | None = None
        self._is_connected = False

    # ------------------------------------------------------------------
    # Connection
    # ------------------------------------------------------------------

    async def _connect(self) -> None:
        """Connect to the Longport API."""
        if self._is_connected:
            self._log.warning("Already connected to Longport execution")
            return

        self._log.info("Connecting to Longport execution API...")

        try:
            await self._instrument_provider.load_all_async()

            lp_config = Config(
                app_key=self._config.get_app_key(),
                app_secret=self._config.get_app_secret(),
                access_token=self._config.get_access_token(),
            )
            self._trade_ctx = TradeContext(lp_config)

            self._is_connected = True
            self._log.info("Connected to Longport execution API", LogColor.GREEN)

        except Exception as e:
            self._log.error(f"Failed to connect to Longport execution API: {e}")
            raise

    async def _disconnect(self) -> None:
        """Disconnect from the Longport API."""
        self._trade_ctx = None
        self._is_connected = False

    def is_connected(self) -> bool:
        return self._is_connected

    # ------------------------------------------------------------------
    # Reconciliation
    # ------------------------------------------------------------------

    async def generate_mass_status(
        self,
        lookback_mins: int | None = None,
    ) -> ExecutionMassStatus | None:
        """Generate ExecutionMassStatus from Longport account state."""
        if not self._trade_ctx:
            self._log.warning("TradeContext not available for reconciliation")
            return None

        self._log.info("Generating ExecutionMassStatus from Longport...")

        try:
            account_id = AccountId(f"LONGPORT-{self._config.account_id}")
            ts_now = self._clock.timestamp_ns()

            mass_status = ExecutionMassStatus(
                client_id=self.id,
                account_id=account_id,
                venue=LONGPORT_VENUE,
                report_id=UUID4(),
                ts_init=ts_now,
            )

            # Fetch today's orders
            orders = self._trade_ctx.today_orders()
            order_reports = []
            for o in orders:
                try:
                    report = self._lp_order_to_report(o, account_id, ts_now)
                    order_reports.append(report)
                except Exception as e:
                    self._log.warning(f"Failed to convert order {o.order_id}: {e}")

            if order_reports:
                mass_status.add_order_reports(order_reports)
                self._log.info(f"Reconciled {len(order_reports)} orders")

            # Fetch today's fills
            executions = self._trade_ctx.today_executions()
            fill_reports = []
            for ex in executions:
                try:
                    report = self._lp_execution_to_fill_report(ex, account_id, ts_now)
                    fill_reports.append(report)
                except Exception as e:
                    self._log.warning(f"Failed to convert execution {ex.trade_id}: {e}")

            if fill_reports:
                mass_status.add_fill_reports(fill_reports)
                self._log.info(f"Reconciled {len(fill_reports)} fills")

            # Fetch stock positions
            positions_resp = self._trade_ctx.stock_positions()
            position_reports = []
            for channel in positions_resp.channels:
                for pos in channel.positions:
                    try:
                        report = self._lp_position_to_report(pos, account_id, ts_now)
                        position_reports.append(report)
                    except Exception as e:
                        self._log.warning(f"Failed to convert position {pos.symbol}: {e}")

            if position_reports:
                mass_status.add_position_reports(position_reports)
                self._log.info(f"Reconciled {len(position_reports)} positions")

            self._log.info("ExecutionMassStatus generated", LogColor.GREEN)
            return mass_status

        except Exception as e:
            self._log.error(f"Failed to generate mass status: {e}")
            return None

    async def generate_order_status_reports(
        self,
        command: GenerateOrderStatusReports,
    ) -> list[OrderStatusReport]:
        if not self._trade_ctx:
            return []
        try:
            account_id = AccountId(f"LONGPORT-{self._config.account_id}")
            ts_now = self._clock.timestamp_ns()
            orders = self._trade_ctx.today_orders()
            return [self._lp_order_to_report(o, account_id, ts_now) for o in orders]
        except Exception as e:
            self._log.error(f"Failed to generate order status reports: {e}")
            return []

    async def generate_fill_reports(
        self,
        command: GenerateFillReports,
    ) -> list[FillReport]:
        if not self._trade_ctx:
            return []
        try:
            account_id = AccountId(f"LONGPORT-{self._config.account_id}")
            ts_now = self._clock.timestamp_ns()
            executions = self._trade_ctx.today_executions()
            return [self._lp_execution_to_fill_report(ex, account_id, ts_now) for ex in executions]
        except Exception as e:
            self._log.error(f"Failed to generate fill reports: {e}")
            return []

    async def generate_position_status_reports(
        self,
        command: GeneratePositionStatusReports,
    ) -> list[PositionStatusReport]:
        if not self._trade_ctx:
            return []
        try:
            account_id = AccountId(f"LONGPORT-{self._config.account_id}")
            ts_now = self._clock.timestamp_ns()
            positions_resp = self._trade_ctx.stock_positions()
            reports = []
            for channel in positions_resp.channels:
                for pos in channel.positions:
                    reports.append(self._lp_position_to_report(pos, account_id, ts_now))
            return reports
        except Exception as e:
            self._log.error(f"Failed to generate position status reports: {e}")
            return []

    # ------------------------------------------------------------------
    # Order execution
    # ------------------------------------------------------------------

    async def _submit_order(self, command) -> None:
        """Submit an order to Longport."""
        if not self._trade_ctx:
            self._log.error("TradeContext not available, cannot submit order")
            return

        order = command.order
        self._log.info(f"Submitting order: {order.client_order_id}")

        try:
            symbol = self._instrument_id_to_lp_symbol(order.instrument_id)
            side = self._map_order_side(order.order_side)
            order_type = self._map_order_type(order.order_type)
            tif = self._map_time_in_force(order.time_in_force)
            qty = Decimal(str(order.quantity))

            kwargs = dict(
                symbol=symbol,
                order_type=order_type,
                side=side,
                submitted_quantity=qty,
                time_in_force=tif,
                remark=str(order.client_order_id),
            )

            if order.order_type in (OrderType.LIMIT, OrderType.STOP_LIMIT, OrderType.LIMIT_IF_TOUCHED):
                if order.price:
                    kwargs["submitted_price"] = Decimal(str(order.price))

            if order.order_type in (OrderType.STOP_MARKET, OrderType.STOP_LIMIT):
                if order.trigger_price:
                    kwargs["trigger_price"] = Decimal(str(order.trigger_price))

            resp = self._trade_ctx.submit_order(**kwargs)
            venue_order_id = VenueOrderId(resp.order_id)
            self._log.info(f"Order submitted: {order.client_order_id} -> {venue_order_id}", LogColor.GREEN)

        except Exception as e:
            self._log.error(f"Failed to submit order {order.client_order_id}: {e}")

    async def _cancel_order(self, command) -> None:
        """Cancel an order on Longport."""
        if not self._trade_ctx:
            self._log.error("TradeContext not available, cannot cancel order")
            return

        order = self._cache.order(command.client_order_id)
        if order is None:
            self._log.error(f"Order not found: {command.client_order_id}")
            return

        if order.venue_order_id is None:
            self._log.error(f"No venue order ID for: {command.client_order_id}")
            return

        try:
            self._trade_ctx.cancel_order(order.venue_order_id.value)
            self._log.info(f"Cancel request sent for: {order.venue_order_id}", LogColor.GREEN)
        except Exception as e:
            self._log.error(f"Failed to cancel order {order.venue_order_id}: {e}")

    async def _modify_order(self, command) -> None:
        """Modify an order on Longport."""
        if not self._trade_ctx:
            self._log.error("TradeContext not available, cannot modify order")
            return

        order = self._cache.order(command.client_order_id)
        if order is None or order.venue_order_id is None:
            self._log.error(f"Order not found or no venue ID: {command.client_order_id}")
            return

        try:
            kwargs = dict(
                order_id=order.venue_order_id.value,
                quantity=Decimal(str(command.quantity)) if command.quantity else Decimal(str(order.quantity)),
            )
            if command.price:
                kwargs["price"] = Decimal(str(command.price))
            if command.trigger_price:
                kwargs["trigger_price"] = Decimal(str(command.trigger_price))

            self._trade_ctx.replace_order(**kwargs)
            self._log.info(f"Modify request sent for: {order.venue_order_id}", LogColor.GREEN)
        except Exception as e:
            self._log.error(f"Failed to modify order {order.venue_order_id}: {e}")

    # ------------------------------------------------------------------
    # Conversion helpers
    # ------------------------------------------------------------------

    def _lp_order_to_report(self, o, account_id: AccountId, ts_now: int) -> OrderStatusReport:
        instrument_id = _lp_symbol_to_instrument_id(o.symbol)
        ts_accepted = _datetime_to_nanos(o.submitted_at)
        ts_last = _datetime_to_nanos(o.updated_at) if o.updated_at else ts_accepted

        price = Price.from_str(str(o.price)) if o.price else None
        trigger_price = Price.from_str(str(o.trigger_price)) if o.trigger_price else None
        avg_px = o.executed_price if o.executed_price else None

        return OrderStatusReport(
            account_id=account_id,
            instrument_id=instrument_id,
            venue_order_id=VenueOrderId(o.order_id),
            order_side=_map_lp_order_side(o.side),
            order_type=_map_lp_order_type(o.order_type),
            time_in_force=_map_lp_time_in_force(o.time_in_force),
            order_status=_map_lp_order_status(o.status),
            quantity=Quantity.from_str(str(o.quantity)),
            filled_qty=Quantity.from_str(str(o.executed_quantity)),
            report_id=UUID4(),
            ts_accepted=ts_accepted or ts_now,
            ts_last=ts_last or ts_now,
            ts_init=ts_now,
            price=price,
            trigger_price=trigger_price,
            avg_px=avg_px,
        )

    def _lp_execution_to_fill_report(self, ex, account_id: AccountId, ts_now: int) -> FillReport:
        instrument_id = _lp_symbol_to_instrument_id(ex.symbol)
        ts_event = _datetime_to_nanos(ex.trade_done_at)

        return FillReport(
            account_id=account_id,
            instrument_id=instrument_id,
            venue_order_id=VenueOrderId(ex.order_id),
            trade_id=TradeId(ex.trade_id),
            order_side=OrderSide.BUY,  # Not available in Execution, default
            last_qty=Quantity.from_str(str(ex.quantity)),
            last_px=Price.from_str(str(ex.price)),
            commission=Money(0.0, self._get_currency(instrument_id)),
            liquidity_side=LiquiditySide.NO_LIQUIDITY_SIDE,
            report_id=UUID4(),
            ts_event=ts_event or ts_now,
            ts_init=ts_now,
        )

    def _lp_position_to_report(self, pos, account_id: AccountId, ts_now: int) -> PositionStatusReport:
        instrument_id = _lp_symbol_to_instrument_id(pos.symbol)
        qty = float(pos.quantity)
        side = PositionSide.LONG if qty > 0 else (PositionSide.SHORT if qty < 0 else PositionSide.FLAT)

        return PositionStatusReport(
            account_id=account_id,
            instrument_id=instrument_id,
            position_side=side,
            quantity=Quantity.from_str(str(abs(qty))),
            report_id=UUID4(),
            ts_last=ts_now,
            ts_init=ts_now,
            avg_px_open=pos.cost_price if hasattr(pos, "cost_price") and pos.cost_price else None,
        )

    def _get_currency(self, instrument_id: InstrumentId):
        """Get currency for an instrument from cache."""
        from nautilus_trader.model.objects import Currency
        instrument = self._cache.instrument(instrument_id)
        if instrument:
            return instrument.quote_currency
        return Currency.from_str("USD")

    def _instrument_id_to_lp_symbol(self, instrument_id: InstrumentId) -> str:
        """Convert Nautilus InstrumentId to Longport symbol string."""
        # e.g. AAPL.US.LONGPORT -> AAPL.US
        parts = instrument_id.value.split(".")
        if len(parts) >= 3:
            return f"{parts[0]}.{parts[1]}"
        return instrument_id.value

    def _map_order_side(self, side: OrderSide):
        from longport.openapi import OrderSide as LPSide
        return LPSide.Buy if side == OrderSide.BUY else LPSide.Sell

    def _map_order_type(self, order_type: OrderType):
        from longport.openapi import OrderType as LPType
        _map = {
            OrderType.LIMIT: LPType.LO,
            OrderType.MARKET: LPType.MO,
            OrderType.STOP_LIMIT: LPType.LIT,
            OrderType.STOP_MARKET: LPType.MIT,
            OrderType.LIMIT_IF_TOUCHED: LPType.LIT,
            OrderType.MARKET_IF_TOUCHED: LPType.MIT,
            OrderType.TRAILING_STOP_LIMIT: LPType.TSLPAMT,
            OrderType.TRAILING_STOP_MARKET: LPType.TSMAMT,
        }
        return _map.get(order_type, LPType.LO)

    def _map_time_in_force(self, tif: TimeInForce):
        from longport.openapi import TimeInForceType as LPTif
        _map = {
            TimeInForce.DAY: LPTif.Day,
            TimeInForce.GTC: LPTif.GTC,
            TimeInForce.GTD: LPTif.GTD,
            TimeInForce.IOC: LPTif.IOC,
            TimeInForce.FOK: LPTif.FOK,
        }
        return _map.get(tif, LPTif.Day)
